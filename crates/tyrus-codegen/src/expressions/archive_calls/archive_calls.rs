use super::*;

impl<'ctx> Codegen<'ctx> {
    /// `pack`, `list` and `unpack` all take their paths first and an optional
    /// options interface last, and all return a `Promise` the runtime fulfils
    /// from its blocking pool with a `Result<T, ArchiveError>`. Failures are
    /// `Err` values, so nothing here guards a synchronous error slot.
    pub(super) fn emit_archive_fn_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if let Some(result) = self.emit_archive_stream_call(callee, args)? {
            return Ok(Some(result));
        }
        let (symbol, name, paths, options_type) = match callee {
            tyrus_core::archive::PACK_TARGET => ("tyrus_archive_pack", "pack", 2, "PackOptions"),
            tyrus_core::archive::LIST_TARGET => ("tyrus_archive_list", "list", 1, "UnpackOptions"),
            tyrus_core::archive::UNPACK_TARGET => {
                ("tyrus_archive_unpack", "unpack", 2, "UnpackOptions")
            }
            _ => return Ok(None),
        };
        if args.len() < paths || args.len() > paths + 1 {
            return Err(format!(
                "error: `{name}` expects {paths} or {} argument(s), found {}",
                paths + 1,
                args.len()
            ));
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let mut strings = Vec::with_capacity(paths);
        for argument in &args[..paths] {
            strings.push(self.emit_expr_as(argument, &Type::Str)?.into_struct_value());
        }
        let options_type = Type::Named(options_type.to_string());
        let options = match args.get(paths) {
            Some(argument) => {
                let value = self.emit_expr_as(argument, &options_type)?;
                let slot = self
                    .builder
                    .build_alloca(self.llvm_type(&options_type), "archive.options")
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_store(slot, value)
                    .map_err(|error| error.to_string())?;
                Some((slot, value, argument))
            }
            None => None,
        };
        let mut parameters: Vec<inkwell::types::BasicTypeEnum<'ctx>> =
            vec![self.string_abi_type(); paths];
        parameters.push(pointer.into());
        let function = self.declare_libc_fn(symbol, &parameters, Some(pointer.into()));
        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
            strings.iter().map(|value| (*value).into()).collect();
        call_args.push(
            options
                .as_ref()
                .map_or_else(|| pointer.const_null(), |(slot, _, _)| *slot)
                .into(),
        );
        let promise = self
            .builder
            .build_call(function, &call_args, &format!("archive.{name}"))
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("internal: `{name}` did not return a promise"))?;
        for (value, argument) in strings.into_iter().zip(&args[..paths]) {
            self.emit_native_string_argument_release(value, argument)?;
        }
        if let Some((_, value, argument)) = options {
            self.emit_async_cleanup_source_expression(value, &options_type, argument)?;
        }
        Ok(Some(promise))
    }

    fn emit_archive_options(
        &mut self,
        argument: Option<&Expr>,
        options_type: &Type,
    ) -> Result<Option<(PointerValue<'ctx>, BasicValueEnum<'ctx>)>, String> {
        let Some(argument) = argument else {
            return Ok(None);
        };
        let value = self.emit_expr_as(argument, options_type)?;
        let slot = self
            .builder
            .build_alloca(self.llvm_type(options_type), "archive.options")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(slot, value)
            .map_err(|error| error.to_string())?;
        Ok(Some((slot, value)))
    }

    /// `packStream(source, format, options?)` returns an owned
    /// `ReadableStream<u8[]>`; `unpackStream(stream, destination, options?)`
    /// borrows its stream, which the runtime retains for as long as it reads.
    fn emit_archive_stream_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let (name, options_type) = match callee {
            tyrus_core::archive::PACK_STREAM_TARGET => ("packStream", "PackOptions"),
            tyrus_core::archive::UNPACK_STREAM_TARGET => ("unpackStream", "UnpackOptions"),
            _ => return Ok(None),
        };
        if !(2..=3).contains(&args.len()) {
            return Err(format!(
                "error: `{name}` expects 2 or 3 argument(s), found {}",
                args.len()
            ));
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let byte_stream = Type::Apply {
            name: "ReadableStream".to_string(),
            args: vec![Type::Array(Box::new(Type::U8))],
        };
        let options_type = Type::Named(options_type.to_string());
        if callee == tyrus_core::archive::PACK_STREAM_TARGET {
            let source = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
            let format = self
                .emit_expr_as(&args[1], &Type::Named("ArchiveFormat".to_string()))?
                .into_int_value();
            let options = self.emit_archive_options(args.get(2), &options_type)?;
            let function = self.declare_libc_fn(
                "tyrus_archive_pack_stream",
                &[
                    self.string_abi_type(),
                    self.context.i32_type().into(),
                    pointer.into(),
                ],
                Some(pointer.into()),
            );
            let stream = self
                .builder
                .build_call(
                    function,
                    &[
                        source.into(),
                        format.into(),
                        options
                            .map_or_else(|| pointer.const_null(), |(slot, _)| slot)
                            .into(),
                    ],
                    "archive.pack_stream",
                )
                .map_err(|error| error.to_string())?
                .try_as_basic_value()
                .basic()
                .ok_or("internal: `packStream` did not return a stream")?;
            self.emit_native_string_argument_release(source, &args[0])?;
            if let (Some((_, value)), Some(argument)) = (options, args.get(2)) {
                self.emit_async_cleanup_source_expression(value, &options_type, argument)?;
            }
            self.record_owned_native_result(stream, byte_stream);
            return Ok(Some(stream));
        }
        let stream = self.emit_expr_as(&args[0], &byte_stream)?;
        let destination = self.emit_expr_as(&args[1], &Type::Str)?.into_struct_value();
        let options = self.emit_archive_options(args.get(2), &options_type)?;
        let function = self.declare_libc_fn(
            "tyrus_archive_unpack_stream",
            &[pointer.into(), self.string_abi_type(), pointer.into()],
            Some(pointer.into()),
        );
        let promise = self
            .builder
            .build_call(
                function,
                &[
                    stream.into(),
                    destination.into(),
                    options
                        .map_or_else(|| pointer.const_null(), |(slot, _)| slot)
                        .into(),
                ],
                "archive.unpack_stream",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or("internal: `unpackStream` did not return a promise")?;
        // The runtime took its own reference. An owned temporary, such as a
        // `packStream(...)` written straight into the call, is released here:
        // inside an async function it is not tracked as a pending result.
        if let Some(plan) = self.take_owned_native_result(stream) {
            self.emit_owned_native_release(stream, &byte_stream, &plan);
        } else if self.owned_stream_capability_expression(&args[0]) {
            let free = self.declare_libc_fn("tyrus_readable_stream_free", &[pointer.into()], None);
            self.builder
                .build_call(free, &[stream.into()], "archive.unpack_stream.release")
                .map_err(|error| error.to_string())?;
        }
        self.emit_native_string_argument_release(destination, &args[1])?;
        if let (Some((_, value)), Some(argument)) = (options, args.get(2)) {
            self.emit_async_cleanup_source_expression(value, &options_type, argument)?;
        }
        Ok(Some(promise))
    }
}
