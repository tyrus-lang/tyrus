use super::*;

enum CompressInput<'ctx> {
    Text(inkwell::values::StructValue<'ctx>),
    Value(BasicValueEnum<'ctx>, Type),
}

impl<'ctx> Codegen<'ctx> {
    /// `compress(data, format, options?)` and `decompress(data, options?)`
    /// hand the runtime a pointer and a length over the input bytes, which it
    /// copies before returning, and resolve with a `Result<u8[], CompressError>`.
    pub(super) fn emit_compress_fn_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if let Some(stream) = self.emit_compress_stream_call(callee, args)? {
            return Ok(Some(stream));
        }
        let (symbol, name, fixed, options_type) = match callee {
            tyrus_core::compress::COMPRESS_TARGET => {
                ("tyrus_compress_compress", "compress", 2, "CompressOptions")
            }
            tyrus_core::compress::DECOMPRESS_TARGET => (
                "tyrus_compress_decompress",
                "decompress",
                1,
                "DecompressOptions",
            ),
            _ => return Ok(None),
        };
        if args.len() < fixed || args.len() > fixed + 1 {
            return Err(format!(
                "error: `{name}` expects {fixed} or {} argument(s), found {}",
                fixed + 1,
                args.len()
            ));
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let (data, length, source) = self.emit_compress_input(&args[0])?;
        let format = if fixed == 2 {
            Some(
                self.emit_expr_as(&args[1], &Type::Named("CompressFormat".to_string()))?
                    .into_int_value(),
            )
        } else {
            None
        };
        let options_type = Type::Named(options_type.to_string());
        let options = match args.get(fixed) {
            Some(argument) => {
                let value = self.emit_expr_as(argument, &options_type)?;
                let slot = self
                    .builder
                    .build_alloca(self.llvm_type(&options_type), "compress.options")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(slot, value)
                    .map_err(|error| error.to_string())?;
                Some((slot, value, argument))
            }
            None => None,
        };
        let mut parameters: Vec<inkwell::types::BasicTypeEnum<'ctx>> =
            vec![pointer.into(), self.usize_type().into()];
        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
            vec![data.into(), length.into()];
        if let Some(format) = format {
            parameters.push(self.context.i32_type().into());
            call_args.push(format.into());
        }
        parameters.push(pointer.into());
        call_args.push(
            options
                .as_ref()
                .map_or_else(|| pointer.const_null(), |(slot, _, _)| *slot)
                .into(),
        );
        let function = self.declare_libc_fn(symbol, &parameters, Some(pointer.into()));
        let promise = self
            .builder
            .build_call(function, &call_args, &format!("compress.{name}"))
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("internal: `{name}` did not return a promise"))?;
        match source {
            CompressInput::Text(text) => {
                self.emit_native_string_argument_release(text, &args[0])?;
            }
            CompressInput::Value(value, ty) => {
                self.emit_async_cleanup_source_expression(value, &ty, &args[0])?;
            }
        }
        if let Some((_, value, argument)) = options {
            self.emit_async_cleanup_source_expression(value, &options_type, argument)?;
        }
        Ok(Some(promise))
    }

    /// `compressionStream(format, options?)` and `decompressionStream(options?)`
    /// return an owned `TransformStream<u8[], u8[]>` handle.
    fn emit_compress_stream_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let (symbol, name, fixed, options_type) = match callee {
            tyrus_core::compress::COMPRESSION_STREAM_TARGET => (
                "tyrus_compress_compression_stream",
                "compressionStream",
                1,
                "CompressOptions",
            ),
            tyrus_core::compress::DECOMPRESSION_STREAM_TARGET => (
                "tyrus_compress_decompression_stream",
                "decompressionStream",
                0,
                "DecompressOptions",
            ),
            _ => return Ok(None),
        };
        if args.len() < fixed || args.len() > fixed + 1 {
            return Err(format!(
                "error: `{name}` expects {fixed} or {} argument(s), found {}",
                fixed + 1,
                args.len()
            ));
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let mut parameters: Vec<inkwell::types::BasicTypeEnum<'ctx>> = Vec::new();
        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = Vec::new();
        if fixed == 1 {
            let format = self
                .emit_expr_as(&args[0], &Type::Named("CompressFormat".to_string()))?
                .into_int_value();
            parameters.push(self.context.i32_type().into());
            call_args.push(format.into());
        }
        let options_type = Type::Named(options_type.to_string());
        let options = match args.get(fixed) {
            Some(argument) => {
                let value = self.emit_expr_as(argument, &options_type)?;
                let slot = self
                    .builder
                    .build_alloca(self.llvm_type(&options_type), "compress.stream.options")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(slot, value)
                    .map_err(|error| error.to_string())?;
                Some((slot, value, argument))
            }
            None => None,
        };
        parameters.push(pointer.into());
        call_args.push(
            options
                .as_ref()
                .map_or_else(|| pointer.const_null(), |(slot, _, _)| *slot)
                .into(),
        );
        let function = self.declare_libc_fn(symbol, &parameters, Some(pointer.into()));
        let stream = self
            .builder
            .build_call(function, &call_args, &format!("compress.{name}"))
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("internal: `{name}` did not return a stream"))?;
        if let Some((_, value, argument)) = options {
            self.emit_async_cleanup_source_expression(value, &options_type, argument)?;
        }
        self.record_owned_native_result(stream, tyrus_core::compress::byte_transform_stream_type());
        Ok(Some(stream))
    }

    /// The bytes of a `u8[]`, or the UTF-8 bytes of a `string`. A value whose
    /// static type is the `u8[] | string` union picks its variant at run time.
    /// The emitted value comes back too, so the caller releases an owned
    /// temporary once the runtime has copied its bytes.
    fn emit_compress_input(
        &mut self,
        argument: &Expr,
    ) -> Result<(PointerValue<'ctx>, IntValue<'ctx>, CompressInput<'ctx>), String> {
        let bytes = Type::Array(Box::new(Type::U8));
        let ty = self
            .expr_type_hint(argument)
            .map(|ty| self.resolve_type(&ty))
            .unwrap_or_else(|| bytes.clone());
        match transparent_codegen_type(&ty).clone() {
            Type::Str => {
                let text = self.emit_expr_as(argument, &Type::Str)?.into_struct_value();
                Ok((
                    self.string_data_ptr(text)?,
                    self.string_byte_len(text)?,
                    CompressInput::Text(text),
                ))
            }
            Type::Union(variants) => {
                let value = self.emit_expr_as(argument, &ty)?;
                let slot = self.native_value_slot(self.llvm_type(&ty), value, "compress.input")?;
                let tag = self.load_union_tag(&variants, slot)?;
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: `compress` outside a function")?;
                let done = self
                    .context
                    .append_basic_block(function, "compress.input.done");
                let mut incoming = Vec::with_capacity(variants.len());
                let mut cases = Vec::with_capacity(variants.len());
                for (index, variant) in variants.iter().enumerate() {
                    let block = self
                        .context
                        .append_basic_block(function, "compress.input.variant");
                    cases.push((self.context.i8_type().const_int(index as u64, false), block));
                    let current = self.builder.get_insert_block();
                    self.builder.position_at_end(block);
                    let payload = self.load_union_payload(
                        &variants,
                        slot,
                        variant,
                        "compress.input.payload",
                    )?;
                    let (data, length) = match transparent_codegen_type(variant) {
                        Type::Str => {
                            let text = payload.into_struct_value();
                            (self.string_data_ptr(text)?, self.string_byte_len(text)?)
                        }
                        _ => self.array_data_and_length(payload.into_struct_value()),
                    };
                    incoming.push((data, length, self.builder.get_insert_block()));
                    self.builder
                        .build_unconditional_branch(done)
                        .map_err(|error| error.to_string())?;
                    if let Some(current) = current {
                        self.builder.position_at_end(current);
                    }
                }
                let fallback = cases[0].1;
                self.builder
                    .build_switch(tag, fallback, &cases)
                    .map_err(|error| error.to_string())?;
                self.builder.position_at_end(done);
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let data = self
                    .builder
                    .build_phi(pointer, "compress.input.data")
                    .map_err(|error| error.to_string())?;
                let length = self
                    .builder
                    .build_phi(self.usize_type(), "compress.input.length")
                    .map_err(|error| error.to_string())?;
                for (pointer_value, length_value, block) in &incoming {
                    let block = block.ok_or("internal: union input block is missing")?;
                    data.add_incoming(&[(pointer_value, block)]);
                    length.add_incoming(&[(length_value, block)]);
                }
                Ok((
                    data.as_basic_value().into_pointer_value(),
                    length.as_basic_value().into_int_value(),
                    CompressInput::Value(value, ty),
                ))
            }
            _ => {
                let array = self.emit_expr_as(argument, &bytes)?;
                let (data, length) = self.array_data_and_length(array.into_struct_value());
                Ok((data, length, CompressInput::Value(array, bytes)))
            }
        }
    }

    fn array_data_and_length(
        &self,
        array: inkwell::values::StructValue<'ctx>,
    ) -> (PointerValue<'ctx>, IntValue<'ctx>) {
        let data = self
            .builder
            .build_extract_value(array, 0, "compress.array.data")
            .unwrap()
            .into_pointer_value();
        let length = self
            .builder
            .build_extract_value(array, 1, "compress.array.length")
            .unwrap()
            .into_int_value();
        (data, length)
    }
}
