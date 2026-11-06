use super::*;

#[path = "statistics_calls/statistics_calls.rs"]
mod statistics_calls;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_math_fn_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if let Some(result) = self.emit_math_statistics_call(callee, args)? {
            return Ok(Some(result));
        }
        match callee {
            "tyrus_math_abs" | "tyrus_math_sign" | "tyrus_math_sqrt" | "tyrus_math_cbrt"
            | "tyrus_math_floor" | "tyrus_math_ceil" | "tyrus_math_trunc" | "tyrus_math_round"
            | "tyrus_math_sin" | "tyrus_math_cos" | "tyrus_math_tan" | "tyrus_math_asin"
            | "tyrus_math_acos" | "tyrus_math_atan" | "tyrus_math_sinh" | "tyrus_math_cosh"
            | "tyrus_math_tanh" | "tyrus_math_exp" | "tyrus_math_log" | "tyrus_math_log10"
            | "tyrus_math_log2" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `{}` expects 1 argument, found {}",
                        callee.strip_prefix("tyrus_math_").unwrap(),
                        args.len()
                    )))
                    .map(Some);
                }
                let x = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    callee,
                    &[self.context.f64_type().into()],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[x.into()], callee)
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_atan2" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `atan2` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let y = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let x = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_atan2",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[y.into(), x.into()], "math.atan2")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_pow" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `pow` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let base = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let exp = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_pow",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[base.into(), exp.into()], "math.pow")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_max" | "tyrus_math_min" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `{}` expects 2 arguments, found {}",
                        callee.strip_prefix("tyrus_math_").unwrap(),
                        args.len()
                    )))
                    .map(Some);
                }
                let a = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let b = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    callee,
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[a.into(), b.into()], callee)
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_imul" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `imul` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let a = self.emit_expr_as(&args[0], &Type::I32)?.into_int_value();
                let b = self.emit_expr_as(&args[1], &Type::I32)?.into_int_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_imul",
                    &[
                        self.context.i32_type().into(),
                        self.context.i32_type().into(),
                    ],
                    Some(self.context.i32_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[a.into(), b.into()], "math.imul")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_clz32" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `clz32` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let x = self.emit_expr_as(&args[0], &Type::I32)?.into_int_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_clz32",
                    &[self.context.i32_type().into()],
                    Some(self.context.i32_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[x.into()], "math.clz32")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_approx_equal" => {
                if !(2..=3).contains(&args.len()) {
                    return (Err(format!(
                        "error: `approxEqual` expects 2 or 3 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let a = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let b = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let tolerance_ptr = if args.len() == 3 {
                    let slot = self
                        .builder
                        .build_alloca(self.context.f64_type(), "math.approxEqual.tolerance")
                        .unwrap();
                    let tolerance = self.emit_expr_as(&args[2], &Type::F64)?;
                    self.builder.build_store(slot, tolerance).unwrap();
                    slot.into()
                } else {
                    self.context
                        .ptr_type(inkwell::AddressSpace::default())
                        .const_null()
                        .into()
                };
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_approx_equal",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                        self.context
                            .ptr_type(inkwell::AddressSpace::default())
                            .into(),
                    ],
                    Some(self.context.bool_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(
                            fn_val,
                            &[a.into(), b.into(), tolerance_ptr],
                            "math.approxEqual",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_clamp" => {
                if args.len() != 3 {
                    return (Err(format!(
                        "error: `clamp` expects 3 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let value = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let min = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let max = self.emit_expr_as(&args[2], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_clamp",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(
                            fn_val,
                            &[value.into(), min.into(), max.into()],
                            "math.clamp",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_clamp_min" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `clampMin` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let value = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let min = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_clamp_min",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[value.into(), min.into()], "math.clampMin")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_clamp_max" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `clampMax` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let value = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let max = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_clamp_max",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[value.into(), max.into()], "math.clampMax")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_lerp" => {
                if args.len() != 3 {
                    return (Err(format!(
                        "error: `lerp` expects 3 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let start = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let end = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let amount = self.emit_expr_as(&args[2], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_lerp",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(
                            fn_val,
                            &[start.into(), end.into(), amount.into()],
                            "math.lerp",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_inverse_lerp" => {
                if args.len() != 3 {
                    return (Err(format!(
                        "error: `inverseLerp` expects 3 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let start = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let end = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let value = self.emit_expr_as(&args[2], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_inverse_lerp",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(
                            fn_val,
                            &[start.into(), end.into(), value.into()],
                            "math.inverseLerp",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_remap" => {
                if args.len() != 5 {
                    return (Err(format!(
                        "error: `remap` expects 5 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let value = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let in_min = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let in_max = self.emit_expr_as(&args[2], &Type::F64)?.into_float_value();
                let out_min = self.emit_expr_as(&args[3], &Type::F64)?.into_float_value();
                let out_max = self.emit_expr_as(&args[4], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_remap",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(
                            fn_val,
                            &[
                                value.into(),
                                in_min.into(),
                                in_max.into(),
                                out_min.into(),
                                out_max.into(),
                            ],
                            "math.remap",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_deg_to_rad" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `degToRad` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let degrees = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_deg_to_rad",
                    &[self.context.f64_type().into()],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[degrees.into()], "math.degToRad")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_rad_to_deg" => {
                if args.len() != 1 {
                    return (Err(format!(
                        "error: `radToDeg` expects 1 argument, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let radians = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_rad_to_deg",
                    &[self.context.f64_type().into()],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[radians.into()], "math.radToDeg")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_gcd" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `gcd` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let a = self.emit_expr_as(&args[0], &Type::U32)?.into_int_value();
                let b = self.emit_expr_as(&args[1], &Type::U32)?.into_int_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_gcd",
                    &[
                        self.context.i32_type().into(),
                        self.context.i32_type().into(),
                    ],
                    Some(self.context.i32_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[a.into(), b.into()], "math.gcd")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_lcm" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `lcm` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let a = self.emit_expr_as(&args[0], &Type::U32)?.into_int_value();
                let b = self.emit_expr_as(&args[1], &Type::U32)?.into_int_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_lcm",
                    &[
                        self.context.i32_type().into(),
                        self.context.i32_type().into(),
                    ],
                    Some(self.context.i32_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[a.into(), b.into()], "math.lcm")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_random_range" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `randomRange` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let min = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let max = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_random_range",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[min.into(), max.into()], "math.randomRange")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_smoothstep" => {
                if args.len() != 3 {
                    return (Err(format!(
                        "error: `smoothstep` expects 3 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let edge0 = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let edge1 = self.emit_expr_as(&args[1], &Type::F64)?.into_float_value();
                let x = self.emit_expr_as(&args[2], &Type::F64)?.into_float_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_smoothstep",
                    &[
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                        self.context.f64_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(
                            fn_val,
                            &[edge0.into(), edge1.into(), x.into()],
                            "math.smoothstep",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap(),
                ));
            }
            "tyrus_math_round_to" => {
                if args.len() != 2 {
                    return (Err(format!(
                        "error: `roundTo` expects 2 arguments, found {}",
                        args.len()
                    )))
                    .map(Some);
                }
                let x = self.emit_expr_as(&args[0], &Type::F64)?.into_float_value();
                let decimals = self.emit_expr_as(&args[1], &Type::I32)?.into_int_value();
                let fn_val = self.declare_libc_fn(
                    "tyrus_math_round_to",
                    &[
                        self.context.f64_type().into(),
                        self.context.i32_type().into(),
                    ],
                    Some(self.context.f64_type().into()),
                );
                return Ok(Some(
                    self.builder
                        .build_call(fn_val, &[x.into(), decimals.into()], "math.roundTo")
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
