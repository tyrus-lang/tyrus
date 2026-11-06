use super::*;

impl<'ctx> Codegen<'ctx> {
    /// The methods that reduce the array to one value or to an index view.
    pub(super) fn emit_array_reduction_method(
        &mut self,
        method: &str,
        args: &[Expr],
        object: &Expr,
        receiver: ArrayReceiver<'_, 'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ArrayReceiver {
            element,
            llvm_array_type,
            pointer,
            array_slot,
            element_size,
            ..
        } = receiver;
        match method {
            "first" | "last" => self.emit_array_extreme(
                array_slot,
                llvm_array_type,
                element,
                element_size,
                method == "last",
            ),
            "sum" | "product" => self.emit_array_numeric_reduction(
                array_slot,
                llvm_array_type,
                element,
                method == "product",
            ),
            "avg" => self.emit_array_average(array_slot, llvm_array_type, element),
            "sortBy" => {
                let view = self.functional_array_view(object)?;
                self.emit_keyed_sort(&view, args, false)
            }
            "sortedBy" => {
                let view = self.functional_array_view(object)?;
                self.emit_keyed_sort(&view, args, true)
            }
            "sumBy" => {
                let view = self.functional_array_view(object)?;
                self.emit_keyed_aggregate(&view, args, false)
            }
            "avgBy" => {
                let view = self.functional_array_view(object)?;
                self.emit_keyed_aggregate(&view, args, true)
            }
            "keys" => {
                let keys_array_type = self.llvm_type(&Type::Array(Box::new(Type::Usize)));
                let output = self
                    .entry_alloca(keys_array_type, "array_keys_out")
                    .unwrap();
                let function = self.declare_libc_fn(
                    "tyrus_array_keys",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(function, &[array_slot.into(), output.into()], "array_keys")
                    .unwrap();
                self.builder
                    .build_load(keys_array_type, output, "array_keys_value")
                    .map_err(|error| error.to_string())
            }
            _ => Err(format!(
                "internal: `{method}` is not an array reductions method"
            )),
        }
    }
}
