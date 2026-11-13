//! The `string` representation, and the only place that knows its shape.
//!
//! A `string` value is `{ptr, i64}`: the UTF-8 bytes, then both lengths packed
//! into one word, the byte length in the low half with the ownership bit at its
//! top, and the character count in the high half. It is bit-for-bit
//! `tyrus_string::TyrusString`, whose documentation carries the reasoning; this
//! module is the code side of that contract.
//!
//! # Why the packed word rather than three fields
//!
//! Because that is what the C ABI does with a sixteen-byte struct, and LLVM IR
//! will not do it for you. A declaration written as `{ptr, i32, i32}` puts the
//! three fields in three registers; a `#[repr(C)]` Rust function reading the
//! same struct expects two. Measured against a Rust staticlib, the mismatch
//! loses the character count with no verifier complaint and no crash, `length`
//! simply answers with a wrong number.
//!
//! Describing the value the way the ABI already wants it removes the coercion
//! from every call site instead of leaving five hundred chances to forget it.
//!
//! Everything that reads or builds a string goes through the helpers here.
//! Nothing else should index field `0` or `1` by hand: when small-string
//! optimization adds an inline variant, this module is what changes.

use super::*;
use inkwell::values::{IntValue, PointerValue, StructValue};

const OWNED: u64 = 1 << 31;

const LENGTH_MASK: u64 = 0xffff_ffff;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn string_type(&self) -> inkwell::types::StructType<'ctx> {
        self.context.struct_type(
            &[
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
                self.context.i64_type().into(),
            ],
            false,
        )
    }

    /// The type a `string` has in a native function's signature.
    ///
    /// The seam that made the flip reviewable: it existed first returning a
    /// bare pointer, so the five-hundred-odd `declare_libc_fn` sites could be
    /// classified, most pass a pointer that is *not* a string, an array, a
    /// runtime handle, a `printf` format, an out-parameter, while every call
    /// was still bit-identical and a wrong answer showed up as a failing test
    /// rather than as a corrupt argument. It now returns the struct, and every
    /// site that was marked follows it.
    ///
    /// A signature that takes a `string` and a signature that takes a pointer
    /// are different types now, so the classification is checked rather than
    /// remembered: the cross-check that compares every runtime signature
    /// against its declaration here is a script, not a review.
    pub(super) fn string_abi_type(&self) -> BasicTypeEnum<'ctx> {
        self.string_type().into()
    }

    pub(super) fn const_string_abi(&self, text: &str, name: &str) -> BasicValueEnum<'ctx> {
        self.const_string_named(text, name).into()
    }

    fn memcmp(&self) -> inkwell::values::FunctionValue<'ctx> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        self.declare_libc_fn(
            "memcmp",
            &[pointer.into(), pointer.into(), self.usize_type().into()],
            Some(self.context.i32_type().into()),
        )
    }

    pub(super) fn emit_string_equals(
        &self,
        left: StructValue<'ctx>,
        right: StructValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let left_len = self.string_byte_len(left)?;
        let right_len = self.string_byte_len(right)?;
        let same_length = self
            .builder
            .build_int_compare(inkwell::IntPredicate::EQ, left_len, right_len, name)
            .map_err(|error| error.to_string())?;
        let same_data = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                self.string_data_ptr(left)?,
                self.string_data_ptr(right)?,
                name,
            )
            .map_err(|error| error.to_string())?;
        let must_compare = self
            .builder
            .build_and(
                same_length,
                self.builder
                    .build_not(same_data, name)
                    .map_err(|error| error.to_string())?,
                name,
            )
            .map_err(|error| error.to_string())?;
        let count = self
            .builder
            .build_select(
                must_compare,
                left_len,
                self.context.i64_type().const_zero(),
                name,
            )
            .map_err(|error| error.to_string())?
            .into_int_value();
        let count = self.truncate_to_usize(count);
        let ordering = self
            .builder
            .build_call(
                self.memcmp(),
                &[
                    self.string_data_ptr(left)?.into(),
                    self.string_data_ptr(right)?.into(),
                    count.into(),
                ],
                name,
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: `memcmp` returned no value")?
            .into_int_value();
        let bytes_match = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                ordering,
                self.context.i32_type().const_zero(),
                name,
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_and(same_length, bytes_match, name)
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_string_compare(
        &self,
        left: StructValue<'ctx>,
        right: StructValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let i32_type = self.context.i32_type();
        let left_len = self.string_byte_len(left)?;
        let right_len = self.string_byte_len(right)?;
        let shorter = self
            .builder
            .build_int_compare(inkwell::IntPredicate::ULT, left_len, right_len, name)
            .map_err(|error| error.to_string())?;
        let common = self
            .builder
            .build_select(shorter, left_len, right_len, name)
            .map_err(|error| error.to_string())?
            .into_int_value();
        let common = self.truncate_to_usize(common);
        let prefix = self
            .builder
            .build_call(
                self.memcmp(),
                &[
                    self.string_data_ptr(left)?.into(),
                    self.string_data_ptr(right)?.into(),
                    common.into(),
                ],
                name,
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: `memcmp` returned no value")?
            .into_int_value();
        let longer = self
            .builder
            .build_int_compare(inkwell::IntPredicate::UGT, left_len, right_len, name)
            .map_err(|error| error.to_string())?;
        let longer_or_equal = self
            .builder
            .build_select(
                longer,
                i32_type.const_int(1, false),
                i32_type.const_zero(),
                name,
            )
            .map_err(|error| error.to_string())?
            .into_int_value();
        let by_length = self
            .builder
            .build_select(shorter, i32_type.const_all_ones(), longer_or_equal, name)
            .map_err(|error| error.to_string())?
            .into_int_value();
        let prefix_ties = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                prefix,
                i32_type.const_zero(),
                name,
            )
            .map_err(|error| error.to_string())?;
        Ok(self
            .builder
            .build_select(prefix_ties, by_length, prefix, name)
            .map_err(|error| error.to_string())?
            .into_int_value())
    }

    fn emit_string_buffer(
        &self,
        bytes: IntValue<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let allocate = self.declare_libc_fn(
            "tyrus_string_alloc",
            &[self.usize_type().into()],
            Some(pointer.into()),
        );
        let bytes = self.truncate_to_usize(bytes);
        Ok(self
            .builder
            .build_call(allocate, &[bytes.into()], name)
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: `tyrus_string_alloc` returned no value")?
            .into_pointer_value())
    }

    pub(super) fn build_owned_string(
        &self,
        buffer: PointerValue<'ctx>,
        bytes: IntValue<'ctx>,
        chars: IntValue<'ctx>,
        name: &str,
    ) -> Result<StructValue<'ctx>, String> {
        let i64_type = self.context.i64_type();
        let high = self
            .builder
            .build_left_shift(chars, i64_type.const_int(32, false), name)
            .map_err(|error| error.to_string())?;
        let low = self
            .builder
            .build_or(bytes, i64_type.const_int(OWNED, false), name)
            .map_err(|error| error.to_string())?;
        let packed = self
            .builder
            .build_or(low, high, name)
            .map_err(|error| error.to_string())?;
        self.build_string(buffer, packed, name)
    }

    pub(super) fn emit_string_clone(
        &self,
        value: StructValue<'ctx>,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let share = self.declare_libc_fn(
            "tyrus_string_share",
            &[self.string_abi_type()],
            Some(self.string_abi_type()),
        );
        Ok(self
            .builder
            .build_call(share, &[value.into()], name)
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: `tyrus_string_share` returned no value")?)
    }

    pub(super) fn emit_string_concat_many(
        &self,
        parts: &[StructValue<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let name = "concat";
        debug_assert!(
            parts.len() >= 2,
            "a concatenation chain has at least two operands"
        );

        let byte_lens = parts
            .iter()
            .map(|part| self.string_byte_len(*part))
            .collect::<Result<Vec<_>, _>>()?;
        let mut total_bytes = byte_lens[0];
        for len in &byte_lens[1..] {
            total_bytes = self
                .builder
                .build_int_add(total_bytes, *len, name)
                .map_err(|error| error.to_string())?;
        }

        let buffer = self.emit_string_buffer(total_bytes, name)?;
        let mut offset: Option<IntValue<'ctx>> = None;
        for (part, len) in parts.iter().zip(&byte_lens) {
            let destination = match offset {
                None => buffer,
                Some(at) => unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), buffer, &[at], name)
                        .map_err(|error| error.to_string())?
                },
            };
            self.builder
                .build_memcpy(destination, 1, self.string_data_ptr(*part)?, 1, *len)
                .map_err(|error| error.to_string())?;
            offset = Some(match offset {
                None => *len,
                Some(at) => self
                    .builder
                    .build_int_add(at, *len, name)
                    .map_err(|error| error.to_string())?,
            });
        }

        let mut total_chars = self.string_char_len(parts[0])?;
        for part in &parts[1..] {
            total_chars = self
                .builder
                .build_int_add(total_chars, self.string_char_len(*part)?, name)
                .map_err(|error| error.to_string())?;
        }

        Ok(self
            .build_owned_string(buffer, total_bytes, total_chars, name)?
            .into())
    }

    pub(super) fn emit_string_concat(
        &self,
        left: StructValue<'ctx>,
        right: StructValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let name = "concat";
        let left_bytes = self.string_byte_len(left)?;
        let right_bytes = self.string_byte_len(right)?;
        let bytes = self
            .builder
            .build_int_add(left_bytes, right_bytes, name)
            .map_err(|error| error.to_string())?;
        let buffer = self.emit_string_buffer(bytes, name)?;
        self.builder
            .build_memcpy(buffer, 1, self.string_data_ptr(left)?, 1, left_bytes)
            .map_err(|error| error.to_string())?;
        let tail = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), buffer, &[left_bytes], name)
                .map_err(|error| error.to_string())?
        };
        self.builder
            .build_memcpy(tail, 1, self.string_data_ptr(right)?, 1, right_bytes)
            .map_err(|error| error.to_string())?;

        let chars = self
            .builder
            .build_int_add(
                self.string_char_len(left)?,
                self.string_char_len(right)?,
                name,
            )
            .map_err(|error| error.to_string())?;
        Ok(self.build_owned_string(buffer, bytes, chars, name)?.into())
    }

    pub(super) fn emit_string_join(
        &self,
        parts: &[StructValue<'ctx>],
        name: &str,
    ) -> Result<StructValue<'ctx>, String> {
        let i64_type = self.context.i64_type();
        if parts.is_empty() {
            return Ok(self.const_string(""));
        }

        let mut bytes = i64_type.const_zero();
        let mut chars = i64_type.const_zero();
        for part in parts {
            bytes = self
                .builder
                .build_int_add(bytes, self.string_byte_len(*part)?, name)
                .map_err(|error| error.to_string())?;
            chars = self
                .builder
                .build_int_add(chars, self.string_char_len(*part)?, name)
                .map_err(|error| error.to_string())?;
        }

        let buffer = self.emit_string_buffer(bytes, name)?;
        let mut offset = i64_type.const_zero();
        for part in parts {
            let part_bytes = self.string_byte_len(*part)?;
            let slot = unsafe {
                self.builder
                    .build_gep(self.context.i8_type(), buffer, &[offset], name)
                    .map_err(|error| error.to_string())?
            };
            self.builder
                .build_memcpy(slot, 1, self.string_data_ptr(*part)?, 1, part_bytes)
                .map_err(|error| error.to_string())?;
            offset = self
                .builder
                .build_int_add(offset, part_bytes, name)
                .map_err(|error| error.to_string())?;
        }

        self.build_owned_string(buffer, bytes, chars, name)
    }

    /// Releases a string's buffer, if the value owns it.
    ///
    /// This is what the ownership bit was for. The old release paths had to
    /// *deduce* whether a pointer came from `.rodata` or from the heap, and
    /// getting that wrong is a crash rather than a leak, the comments in
    /// `arrays.rs` and `methods.rs` are the record of it having happened. Now
    /// the value says so.
    ///
    /// Expressed without a branch: a value that owns nothing yields a null
    /// pointer, and `free(NULL)` is defined to do nothing.
    pub(super) fn emit_string_release(&self, value: StructValue<'ctx>) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let target = self
            .builder
            .build_select(
                self.string_is_owned(value)?,
                self.string_data_ptr(value)?,
                pointer.const_null(),
                "str_release",
            )
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let release = self.declare_libc_fn("tyrus_string_release_buffer", &[pointer.into()], None);
        self.builder
            .build_call(release, &[target.into()], "")
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn packed_lengths(bytes: usize, chars: usize, owned: bool) -> u64 {
        let ownership = if owned { OWNED } else { 0 };
        (bytes as u64 & LENGTH_MASK) | ownership | ((chars as u64 & LENGTH_MASK) << 32)
    }

    pub(super) fn const_string(&self, text: &str) -> StructValue<'ctx> {
        self.const_string_named(text, "str_data")
    }

    pub(super) fn const_string_named(&self, text: &str, name: &str) -> StructValue<'ctx> {
        let bytes = text.as_bytes();
        let interned = matches!(name, "str" | "str_data");
        let global = match interned
            .then(|| self.interned_strings.borrow().get(text).copied())
            .flatten()
        {
            Some(global) => global,
            None => {
                let global = self.module.add_global(
                    self.context.i8_type().array_type(bytes.len() as u32),
                    None,
                    name,
                );
                global.set_initializer(&self.context.const_string(bytes, false));
                global.set_constant(true);
                global.set_linkage(inkwell::module::Linkage::Private);
                global.set_unnamed_addr(true);
                if interned {
                    self.interned_strings
                        .borrow_mut()
                        .insert(text.to_string(), global);
                }
                global
            }
        };

        let packed = Self::packed_lengths(bytes.len(), text.chars().count(), false);
        self.string_type().const_named_struct(&[
            global.as_pointer_value().into(),
            self.context.i64_type().const_int(packed, false).into(),
        ])
    }

    pub(super) fn build_string(
        &self,
        data: PointerValue<'ctx>,
        packed: IntValue<'ctx>,
        name: &str,
    ) -> Result<StructValue<'ctx>, String> {
        let value = self
            .builder
            .build_insert_value(self.string_type().get_undef(), data, 0, name)
            .map_err(|error| error.to_string())?;
        Ok(self
            .builder
            .build_insert_value(value, packed, 1, name)
            .map_err(|error| error.to_string())?
            .into_struct_value())
    }

    pub(super) fn is_string_value(&self, value: BasicValueEnum<'ctx>) -> bool {
        value.is_struct_value() && value.into_struct_value().get_type() == self.string_type()
    }

    pub(super) fn string_data_ptr(
        &self,
        value: StructValue<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        Ok(self
            .builder
            .build_extract_value(value, 0, "str_data")
            .map_err(|error| error.to_string())?
            .into_pointer_value())
    }

    /// Both lengths, as the single word the ABI passes them in.
    ///
    /// The generated code sees a string as `{ptr, i64}`, so the two `u32`
    /// fields arrive as one value and every length question starts here.
    pub(super) fn string_packed_lengths(
        &self,
        value: StructValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        Ok(self
            .builder
            .build_extract_value(value, 1, "str_lengths")
            .map_err(|error| error.to_string())?
            .into_int_value())
    }

    pub(super) fn string_char_len(
        &self,
        value: StructValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let packed = self.string_packed_lengths(value)?;
        self.builder
            .build_right_shift(
                packed,
                self.context.i64_type().const_int(32, false),
                false,
                "str_char_len",
            )
            .map_err(|error| error.to_string())
    }

    pub(super) fn string_byte_len(
        &self,
        value: StructValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let packed = self.string_packed_lengths(value)?;
        self.builder
            .build_and(
                packed,
                self.context
                    .i64_type()
                    .const_int(LENGTH_MASK & !OWNED, false),
                "str_byte_len",
            )
            .map_err(|error| error.to_string())
    }

    pub(super) fn string_is_owned(
        &self,
        value: StructValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let packed = self.string_packed_lengths(value)?;
        let bit = self
            .builder
            .build_and(
                packed,
                self.context.i64_type().const_int(OWNED, false),
                "str_owned_bit",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_int_compare(
                inkwell::IntPredicate::NE,
                bit,
                self.context.i64_type().const_zero(),
                "str_is_owned",
            )
            .map_err(|error| error.to_string())
    }

    pub(super) fn null_string(&self) -> StructValue<'ctx> {
        self.string_type().const_named_struct(&[
            self.context
                .ptr_type(inkwell::AddressSpace::default())
                .const_null()
                .into(),
            self.context.i64_type().const_zero().into(),
        ])
    }

    pub(super) fn string_is_null(
        &self,
        value: StructValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let data = self.string_data_ptr(value)?;
        self.builder
            .build_is_null(data, "str_is_null")
            .map_err(|error| error.to_string())
    }
}
