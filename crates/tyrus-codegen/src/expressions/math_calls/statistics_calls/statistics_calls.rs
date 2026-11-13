use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_math_statistics_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        match callee {
            "tyrus_math_mean"
            | "tyrus_math_median"
            | "tyrus_math_mode"
            | "tyrus_math_variance"
            | "tyrus_math_std_dev" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `{}` expects 1 argument, found {}",
                        callee.strip_prefix("tyrus_math_").unwrap(),
                        args.len()
                    )))
                    .map(Some);
                }
                let array = self
                    .emit_expr_as(&args[0], &Type::Array(Box::new(Type::F64)))?
                    .into_struct_value();
                let fn_name = callee.strip_prefix("tyrus_math_").unwrap();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "math.array.data")
                    .unwrap();
                let length = self
                    .builder
                    .build_extract_value(array, 1, "math.array.len")
                    .unwrap();
                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    callee,
                    &[ptr_ty.into(), self.usize_type().into()],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(
                            fn_val,
                            &[data.into(), length.into()],
                            format!("math.{fn_name}").as_str(),
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_percentile" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `percentile` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let p = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let array = self
                    .emit_expr_as(&args[1], &Type::Array(Box::new(Type::F64)))?
                    .into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "math.array.data")
                    .unwrap();
                let length = self
                    .builder
                    .build_extract_value(array, 1, "math.array.len")
                    .unwrap();
                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_percentile",
                    &[
                        self.context.f64_type().into(),
                        ptr_ty.into(),
                        self.usize_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(
                            fn_val,
                            &[p.into(), data.into(), length.into()],
                            "math.percentile",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_factorial" | "tyrus_math_permutation" | "tyrus_math_combination" => {
                let (arg1, arg2) = match args.len() {
                    1 => (
                        self.emit_expr_as(&args[0], &Type::U128)?.into_int_value(),
                        None,
                    ),
                    2 => (
                        self.emit_expr_as(&args[0], &Type::U128)?.into_int_value(),
                        Some(self.emit_expr_as(&args[1], &Type::U128)?.into_int_value()),
                    ),
                    _ => {
                        return (Err(format!(
                            "error: `{}` expects 1 or 2 arguments, found {}",
                            callee.strip_prefix("tyrus_math_").unwrap(),
                            args.len()
                        )))
                        .map(Some);
                    }
                };
                let fn_name = callee.strip_prefix("tyrus_math_").unwrap();
                let mut param_types = vec![self.context.i128_type().into()];
                if arg2.is_some() {
                    param_types.push(self.context.i128_type().into());
                }
                let fn_val = self.declare_libc_fn(
                    callee,
                    &param_types,
                    Some(self.context.i128_type().into()),
                );
                let mut call_args = vec![arg1.into()];
                if let Some(arg2) = arg2 {
                    call_args.push(arg2.into());
                }
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &call_args, format!("math.{fn_name}").as_str())
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_is_prime" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `isPrime` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let n = self.emit_expr_as(&args[0], &Type::U64)?.into_int_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_is_prime",
                    &[self.context.i64_type().into()],
                    Some(self.context.bool_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[n.into()], "math.isPrime")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_next_prime" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `nextPrime` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let n = self.emit_expr_as(&args[0], &Type::U64)?.into_int_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_next_prime",
                    &[self.context.i64_type().into()],
                    Some(self.context.i64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[n.into()], "math.nextPrime")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_prime_factors" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `primeFactors` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let n = self.emit_expr_as(&args[0], &Type::U64)?.into_int_value();
                let array_ty = self.llvm_type(&Type::Array(Box::new(Type::U64)));
                let slot = self
                    .builder
                    .build_alloca(array_ty, "math.primeFactors.slot")
                    .unwrap();
                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_prime_factors",
                    &[self.context.i64_type().into(), ptr_ty.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[n.into(), slot.into()], "math.primeFactors.call")
                    .unwrap();
                let value = self
                    .builder
                    .build_load(array_ty, slot, "math.primeFactors")
                    .unwrap();
                self.record_owned_native_result(value, Type::Array(Box::new(Type::U64)));
                return Ok(Some(value));
            }
            "tyrus_math_fibonacci" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `fibonacci` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let n = self.emit_expr_as(&args[0], &Type::U64)?.into_int_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_fibonacci",
                    &[self.context.i64_type().into()],
                    Some(self.context.i64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[n.into()], "math.fibonacci")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_hypot" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `hypot` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let x = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let y = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_hypot",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[x.into(), y.into()], "math.hypot")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_copysign" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `copysign` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let x = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let y = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_copysign",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[x.into(), y.into()], "math.copysign")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_fmod" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `fmod` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let x = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let y = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_fmod",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[x.into(), y.into()], "math.fmod")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_log_base" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `logBase` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let x = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let base = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_log_base",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[x.into(), base.into()], "math.logBase")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_powi" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `powi` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let x = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let exp = self.emit_expr_as(&args[1], &Type::I32)?.into_int_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_powi",
                    &[
                        self.context.f64_type().into(),
                        self.context.i32_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[x.into(), exp.into()], "math.powi")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_to_polar" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `toPolar` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let x = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let y = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_to_polar",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.llvm_type(&Type::Tuple(vec![Type::F64, Type::F64]))),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[x.into(), y.into()], "math.toPolar")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_to_cartesian" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `toCartesian` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let r = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let theta = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_to_cartesian",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.llvm_type(&Type::Tuple(vec![Type::F64, Type::F64]))),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[r.into(), theta.into()], "math.toCartesian")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            _ => {}
        }
        Ok(None)
    }
}
