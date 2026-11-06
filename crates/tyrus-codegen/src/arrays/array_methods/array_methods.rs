#[path = "derivations/derivations.rs"]
mod derivations;
#[path = "mutations/mutations.rs"]
mod mutations;
#[path = "reductions/reductions.rs"]
mod reductions;
#[path = "rewrites/rewrites.rs"]
mod rewrites;
#[path = "sampling/sampling.rs"]
mod sampling;
#[path = "searches/searches.rs"]
mod searches;
#[path = "toggling/toggling.rs"]
mod toggling;
use super::*;

/// What `emit_array_method_inner` settles about the receiver before it
/// dispatches: every group of methods below reads these instead of recomputing
/// them, and every one of them is `Copy`, so passing the set costs nothing and
/// never fights the `&mut self` the emission needs.
#[derive(Clone, Copy)]
pub(super) struct ArrayReceiver<'a, 'ctx> {
    pub(super) array_type: &'a Type,
    pub(super) element: &'a Type,
    pub(super) llvm_array_type: BasicTypeEnum<'ctx>,
    pub(super) pointer: inkwell::types::PointerType<'ctx>,
    pub(super) i64_type: inkwell::types::IntType<'ctx>,
    pub(super) i32_type: inkwell::types::IntType<'ctx>,
    pub(super) array_slot: PointerValue<'ctx>,
    pub(super) element_size: inkwell::values::IntValue<'ctx>,
    pub(super) owns_string_elements: bool,
    pub(super) appends_owned_string_elements: bool,
}

impl<'ctx> Codegen<'ctx> {
    fn array_method_value_ptr(
        &mut self,
        value: &Expr,
        element: &Type,
    ) -> Result<PointerValue<'ctx>, String> {
        let slot = self
            .entry_alloca(self.llvm_type(element), "array_method_value")
            .unwrap();
        let value = self.emit_expr_as(value, element)?;
        self.builder.build_store(slot, value).unwrap();
        Ok(slot)
    }
    pub(super) fn array_method_changes_length(method: &str) -> bool {
        matches!(method, "push" | "unshift" | "pop" | "shift" | "splice")
    }

    pub(crate) fn emit_array_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let result = self.emit_array_method_inner(object, method, args)?;
        if Self::array_method_changes_length(method)
            && let ExprKind::FieldAccess {
                object: owner,
                field,
            } = object.kind()
            && let Some((decorators, ty)) = self.field_decorators(owner, field)
        {
            let mutated = self.emit_expr(object)?;
            let owns_collection = self.validation_depth == 0;
            if owns_collection {
                self.emit_validation_issues_begin();
            }
            self.emit_validator_checks(mutated, &ty, &decorators, field)?;
            if owns_collection {
                self.emit_validation_issues_finish();
            }
        }
        Ok(result)
    }

    pub(super) fn emit_array_method_inner(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if is_functional_array_method(method) {
            return self.emit_functional_array_method(object, method, args, None);
        }
        let borrowed = self.tuple_slice(object, "tuple_receiver")?;
        let (array_type, element_owned, array_slot_borrowed) = match borrowed {
            Some(slice) => {
                let array_type = Type::Array(Box::new(slice.element_type.clone()));
                let view = self.borrowed_array_view(slice, "tuple_receiver_view")?;
                (array_type, Some(view.element_type), Some(view.slot))
            }
            None => (
                self.expr_type_hint(object)
                    .ok_or_else(|| "error: cannot infer array receiver type".to_string())?,
                None,
                None,
            ),
        };
        let element_from_type;
        let element = match &element_owned {
            Some(element) => element,
            None => {
                let Type::Array(element) = transparent_codegen_type(&array_type) else {
                    return Err(format!(
                        "error: method requires an array receiver, found `{array_type}`"
                    ));
                };
                element_from_type = element.as_ref().clone();
                &element_from_type
            }
        };
        let element = &element.clone();
        let llvm_array_type = self.llvm_type(&array_type);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();
        let i32_type = self.context.i32_type();
        let (array_slot, temporary_receiver) = if let Some(slot) = array_slot_borrowed {
            (slot, None)
        } else if let ExprKind::Ident(name) = object.kind() {
            (
                self.load_var(name)
                    .map(|(slot, _)| slot)
                    .ok_or_else(|| format!("internal: array `{name}` was not found"))?,
                None,
            )
        } else if matches!(object.kind(), ExprKind::FieldAccess { .. }) {
            (self.emit_place_ptr(object)?.0, None)
        } else {
            let value = self.emit_expr(object)?;
            let slot = self
                .entry_alloca(llvm_array_type, "array_receiver")
                .unwrap();
            self.builder.build_store(slot, value).unwrap();
            (slot, Some(value))
        };
        let element_size = self
            .llvm_type(element)
            .size_of()
            .ok_or_else(|| format!("error: array element type `{element}` has no native size"))?;
        let async_owned_string_array =
            if matches!(
                method,
                "push" | "unshift" | "pop" | "shift" | "reverse" | "fill" | "copyWithin" | "splice"
            ) && matches!(transparent_codegen_type(element), Type::Str)
            {
                match object.kind() {
                    ExprKind::Ident(name) => self
                        .async_frame_string_array_ownership(name)
                        .map(|ownership| (name.as_str(), ownership)),
                    _ => None,
                }
            } else {
                None
            };
        let owned_object_string_array = matches!(transparent_codegen_type(element), Type::Str)
            && tyrus_core::ast::place_root_ident(object)
                .is_some_and(|name| self.async_frame_owns_value(name));
        // The synchronous counterpart of `async_owned_string_array`: a
        // `string[]` local whose ownership mode is tracked at run time by
        // `string_array_ownership`. Mutating one promotes it to mode `2` first,
        // so every element it holds afterwards is an allocation the array owns,
        // which is what lets the mutation run in its owned-string form and
        // what lets scope exit release the whole array.
        //
        // Before this, a mutation outside an async frame copied the argument's
        // 8 raw pointer bytes into the array and left ownership untouched, so a
        // freshly computed element (`parts.push("item-" + i.toString())`) was
        // released as an ordinary statement temporary right after the call and
        // the array kept a dangling pointer, a use-after-free that read
        // recycled heap bytes and segfaulted in `strlen` once anything walked
        // the element later.
        // `pop`/`shift` normalize for a different reason than the others: they
        // hand an element *out* of the array, and the caller can only be given
        // ownership of it if it is an allocation rather than one of the
        // literal's `.rodata` pointers. Promoting first makes the removal
        // uniform, so the result is always the caller's to release.
        let sync_tracked_string_array =
            if matches!(
                method,
                "push" | "unshift" | "fill" | "copyWithin" | "splice" | "pop" | "shift"
            ) && matches!(transparent_codegen_type(element), Type::Str)
                && async_owned_string_array.is_none()
                && !owned_object_string_array
            {
                match object.kind() {
                    ExprKind::Ident(name) => self
                        .string_array_ownership_slot(name)
                        .map(|mode_slot| (name.clone(), mode_slot)),
                    _ => None,
                }
            } else {
                None
            };
        if let Some((name, mode_slot)) = &sync_tracked_string_array {
            let function = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
                .ok_or("internal: string-array mutation outside function")?;
            self.normalize_string_array_for_mutation(
                function,
                name,
                array_slot,
                *mode_slot,
                &array_type,
            )?;
        }
        let owns_string_elements = async_owned_string_array.is_some()
            || owned_object_string_array
            || sync_tracked_string_array.is_some();
        let appends_owned_string_elements =
            owns_string_elements || matches!(transparent_codegen_type(element), Type::Str);
        if let Some((name, ownership)) = async_owned_string_array {
            let function = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
                .ok_or("internal: async string-array mutation outside function")?;
            self.normalize_async_string_array_for_mutation(
                function,
                name,
                array_slot,
                ownership,
                &array_type,
            )?;
        }

        let receiver = ArrayReceiver {
            array_type: &array_type,
            element,
            llvm_array_type,
            pointer,
            i64_type,
            i32_type,
            array_slot,
            element_size,
            owns_string_elements,
            appends_owned_string_elements,
        };

        let result = match method {
            "push" | "unshift" | "pop" | "shift" | "splice" => {
                self.emit_array_mutation_method(method, args, receiver)
            }
            "equals" | "includes" | "indexOf" | "lastIndexOf" => {
                self.emit_array_search_method(method, args, receiver)
            }
            "reverse" | "fill" | "copyWithin" | "rotate" => {
                self.emit_array_rewrite_method(method, args, receiver)
            }
            "at" | "random" | "shuffle" | "sample" => {
                self.emit_array_sampling_method(method, args, receiver)
            }
            "first" | "last" | "sum" | "product" | "avg" | "sortBy" | "sortedBy" | "sumBy"
            | "avgBy" | "keys" => self.emit_array_reduction_method(method, args, object, receiver),
            "entries" | "slice" | "concat" | "toReversed" | "with" | "values" | "splitAt"
            | "join" => self.emit_array_derivation_method(method, args, receiver),
            "toggle" | "toggleAll" => self.emit_array_toggle_method(method, args, receiver),
            _ => Err(format!("error: unsupported array method `{method}`")),
        };
        if result.is_ok()
            && let Some(receiver) = temporary_receiver
        {
            self.emit_async_cleanup_source_expression(receiver, &array_type, object)?;
        }
        result
    }
}
