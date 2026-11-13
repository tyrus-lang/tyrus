use super::*;

impl<'ctx> Codegen<'ctx> {
    fn emit_algorithm_result(
        &mut self,
        success_type: Type,
        success_value: BasicValueEnum<'ctx>,
        label: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let error_type = Type::Named("AlgorithmError".to_string());
        let result_type = Type::Result {
            success: Box::new(success_type.clone()),
            error: Box::new(error_type.clone()),
        };
        let variants = vec![success_type, error_type.clone()];
        let union_type = self.union_llvm_type(&variants);
        let result_slot = self.entry_alloca(union_type, &format!("{label}.result"))?;
        self.builder
            .build_store(result_slot, union_type.const_zero())
            .map_err(|error| error.to_string())?;

        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let string_type = self.string_abi_type();
        let taken_type = self.context.struct_type(&[string_type, string_type], false);
        let taken_slot = self.entry_alloca(taken_type, &format!("{label}.error"))?;
        self.builder
            .build_store(taken_slot, taken_type.const_zero())
            .map_err(|error| error.to_string())?;
        let take_error =
            self.declare_libc_fn("tyrus_algorithm_take_error", &[pointer.into()], None);
        self.builder
            .build_call(take_error, &[taken_slot.into()], "")
            .map_err(|error| error.to_string())?;
        let code_ptr = self
            .builder
            .build_struct_gep(
                taken_type,
                taken_slot,
                0,
                &format!("{label}.error.code.ptr"),
            )
            .map_err(|error| error.to_string())?;
        let code = self
            .builder
            .build_load(string_type, code_ptr, &format!("{label}.error.code"))
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let failed = self
            .builder
            .build_not(self.string_is_null(code)?, &format!("{label}.failed"))
            .map_err(|error| error.to_string())?;
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: algorithm Result emitted outside a function")?;
        let error_block = self
            .context
            .append_basic_block(function, &format!("{label}.error.variant"));
        let success_block = self
            .context
            .append_basic_block(function, &format!("{label}.ok.variant"));
        let done = self
            .context
            .append_basic_block(function, &format!("{label}.done"));
        self.builder
            .build_conditional_branch(failed, error_block, success_block)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(success_block);
        self.store_union_variant(&variants, result_slot, 0, success_value)?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(error_block);
        let fields = self
            .declared_fields_with_index(&error_type)
            .ok_or("internal: AlgorithmError interface is unavailable")?;
        let mut error_value = self.llvm_type(&error_type).into_struct_type().const_zero();
        for (index, field) in fields {
            let source_index = match field.name.as_str() {
                "code" => 0,
                "message" => 1,
                _ => continue,
            };
            let source_ptr = self
                .builder
                .build_struct_gep(
                    taken_type,
                    taken_slot,
                    source_index,
                    &format!("{label}.error.{}.ptr", field.name),
                )
                .map_err(|error| error.to_string())?;
            let value = self
                .builder
                .build_load(
                    string_type,
                    source_ptr,
                    &format!("{label}.error.{}", field.name),
                )
                .map_err(|error| error.to_string())?;
            error_value = self
                .builder
                .build_insert_value(
                    error_value,
                    value,
                    index,
                    &format!("{label}.error.value.{}", field.name),
                )
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        self.store_union_variant(&variants, result_slot, 1, error_value.into())?;
        self.builder
            .build_unconditional_branch(done)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(done);
        let result = self
            .builder
            .build_load(union_type, result_slot, &format!("{label}.result.value"))
            .map_err(|error| error.to_string())?;
        self.record_owned_native_result(result, result_type);
        Ok(result)
    }

    pub(super) fn emit_path_array_argument(
        &mut self,
        expr: &Expr,
        ty: &Type,
        name: &str,
    ) -> Result<(BasicValueEnum<'ctx>, BasicValueEnum<'ctx>), String> {
        let array = self.emit_expr_as(expr, ty)?.into_struct_value();
        let data = self
            .builder
            .build_extract_value(array, 0, &format!("{name}.data"))
            .map_err(|e| e.to_string())?;
        let length = self
            .builder
            .build_extract_value(array, 1, &format!("{name}.length"))
            .map_err(|e| e.to_string())?;
        Ok((data, length))
    }

    pub(super) fn emit_path_result_optional(
        &mut self,
        result: PointerValue<'ctx>,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let path_ty = Type::Tuple(vec![Type::Array(Box::new(Type::Usize)), Type::I32]);
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| "error: path result is not inside a function".to_string())?;
        let present = self
            .context
            .append_basic_block(function, &format!("{name}.present"));
        let absent = self
            .context
            .append_basic_block(function, &format!("{name}.absent"));
        let merge = self
            .context
            .append_basic_block(function, &format!("{name}.merge"));
        let is_present = self
            .builder
            .build_is_not_null(result, &format!("{name}.present"))
            .map_err(|e| e.to_string())?;
        self.builder
            .build_conditional_branch(is_present, present, absent)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(absent);
        let null = pointer.const_null();
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|e| e.to_string())?;
        let absent_end = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(present);
        let loaded = self
            .builder
            .build_load(self.llvm_type(&path_ty), result, &format!("{name}.value"))
            .map_err(|e| e.to_string())?;
        let boxed = self.emit_optional_box(loaded, &path_ty)?;
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[result.into()], &format!("{name}.free"))
            .map_err(|e| e.to_string())?;
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|e| e.to_string())?;
        let present_end = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(merge);
        let phi = self
            .builder
            .build_phi(pointer, &format!("{name}.optional"))
            .map_err(|e| e.to_string())?;
        phi.add_incoming(&[(&null, absent_end), (&boxed, present_end)]);
        let value = phi.as_basic_value();
        self.record_owned_native_result_with(
            value,
            Type::Optional(Box::new(path_ty)),
            crate::ReleasePlan::Box(Box::new(crate::ReleasePlan::Deep)),
        );
        Ok(value)
    }

    pub(super) fn emit_topological_kahn_info_call(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let result_ty = Type::Named("TopologicalSortKahnInfo".to_string());
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "topological.kahn.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let input_ty = self
            .expr_type_hint(&args[0])
            .ok_or("error: cannot infer topological input type")?;
        let graph_input = matches!(transparent_codegen_type(&input_ty), Type::Graph(_, _));
        let (name, params, call_args) = if graph_input {
            (
                "tyrus_algorithm_topological_sort_kahn_info_graph",
                vec![pointer.into(), pointer.into()],
                vec![self.emit_expr_as(&args[0], &input_ty)?, output.into()],
            )
        } else {
            let (data, length) =
                self.emit_path_array_argument(&args[0], &input_ty, "topological.kahn.info.input")?;
            (
                "tyrus_algorithm_topological_sort_kahn_info",
                vec![pointer.into(), self.usize_type().into(), pointer.into()],
                vec![data, length, output.into()],
            )
        };
        let function = self.declare_libc_fn(name, &params, None);
        let metadata: Vec<BasicMetadataValueEnum<'ctx>> =
            call_args.into_iter().map(Into::into).collect();
        self.builder
            .build_call(function, &metadata, "topological.kahn.info.call")
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(
                self.llvm_type(&result_ty),
                output,
                "topological.kahn.info.result",
            )
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    pub(super) fn emit_topological_dfs_info_call(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let result_ty = Type::Named("TopologicalSortDfsInfo".to_string());
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "topological.dfs.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let input_ty = self
            .expr_type_hint(&args[0])
            .ok_or("error: cannot infer topological input type")?;
        if matches!(transparent_codegen_type(&input_ty), Type::Graph(_, _)) {
            return Err("error: topologicalSortDfsInfo Graph input is not implemented yet".into());
        }
        let (data, length) =
            self.emit_path_array_argument(&args[0], &input_ty, "topological.dfs.info.input")?;
        let function = self.declare_libc_fn(
            "tyrus_algorithm_topological_sort_dfs_info",
            &[pointer.into(), self.usize_type().into(), pointer.into()],
            None,
        );
        self.builder
            .build_call(
                function,
                &[data.into(), length.into(), output.into()],
                "topological.dfs.info.call",
            )
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(
                self.llvm_type(&result_ty),
                output,
                "topological.dfs.info.result",
            )
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    pub(super) fn emit_scc_info_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let (name, result_ty) = match target {
            tyrus_core::algorithm::TARJAN_SCC_INFO_TARGET => (
                "tyrus_algorithm_tarjan_scc_info",
                Type::Named("TarjanSccInfo".to_string()),
            ),
            tyrus_core::algorithm::KOSARAJU_SCC_INFO_TARGET => (
                "tyrus_algorithm_kosaraju_scc_info",
                Type::Named("KosarajuSccInfo".to_string()),
            ),
            _ => return Err("error: unknown SCC info algorithm".into()),
        };
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "scc.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let input_ty = self
            .expr_type_hint(&args[0])
            .ok_or("error: cannot infer SCC input type")?;
        if matches!(transparent_codegen_type(&input_ty), Type::Graph(_, _)) {
            return Err("error: SCC info Graph input is not implemented yet".into());
        }
        let (data, length) =
            self.emit_path_array_argument(&args[0], &input_ty, "scc.info.input")?;
        let function = self.declare_libc_fn(
            name,
            &[pointer.into(), self.usize_type().into(), pointer.into()],
            None,
        );
        self.builder
            .build_call(
                function,
                &[data.into(), length.into(), output.into()],
                "scc.info.call",
            )
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(self.llvm_type(&result_ty), output, "scc.info.result")
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    pub(super) fn emit_connectivity_info_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let (name, result_ty) = match target {
            tyrus_core::algorithm::FIND_BRIDGES_INFO_TARGET => (
                "tyrus_algorithm_find_bridges_info",
                Type::Named("FindBridgesInfo".to_string()),
            ),
            tyrus_core::algorithm::FIND_CUT_VERTICES_INFO_TARGET => (
                "tyrus_algorithm_find_cut_vertices_info",
                Type::Named("FindCutVerticesInfo".to_string()),
            ),
            tyrus_core::algorithm::BICONNECTED_COMPONENTS_INFO_TARGET => (
                "tyrus_algorithm_biconnected_components_info",
                Type::Named("BiconnectedComponentsInfo".to_string()),
            ),
            _ => return Err("error: unknown connectivity info algorithm".into()),
        };
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "connectivity.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let input_ty = self
            .expr_type_hint(&args[0])
            .ok_or("error: cannot infer connectivity input type")?;
        let graph_input = matches!(transparent_codegen_type(&input_ty), Type::Graph(_, _));
        let (name, params, call_args) = if graph_input {
            (
                format!("{name}_graph"),
                vec![pointer.into(), pointer.into()],
                vec![self.emit_expr(&args[0])?, output.into()],
            )
        } else {
            let (data, length) =
                self.emit_path_array_argument(&args[0], &input_ty, "connectivity.info.input")?;
            (
                name.to_string(),
                vec![pointer.into(), self.usize_type().into(), pointer.into()],
                vec![data, length, output.into()],
            )
        };
        let function = self.declare_libc_fn(&name, &params, None);
        let metadata: Vec<BasicMetadataValueEnum<'ctx>> =
            call_args.into_iter().map(Into::into).collect();
        self.builder
            .build_call(function, &metadata, "connectivity.info.call")
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(
                self.llvm_type(&result_ty),
                output,
                "connectivity.info.result",
            )
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    pub(super) fn emit_matching_info_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let (base, result_ty, graph_supported) = match target {
            tyrus_core::algorithm::EDMONDS_BLOSSOM_INFO_TARGET => (
                "tyrus_algorithm_edmonds_blossom_info",
                Type::Named("EdmondsBlossomInfo".to_string()),
                true,
            ),
            tyrus_core::algorithm::HUNGARIAN_INFO_TARGET => (
                "tyrus_algorithm_hungarian_info",
                Type::Named("HungarianInfo".to_string()),
                false,
            ),
            _ => return Err("error: unknown matching info algorithm".into()),
        };
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "matching.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let input_ty = self
            .expr_type_hint(&args[0])
            .ok_or("error: cannot infer matching input type")?;
        let graph = matches!(transparent_codegen_type(&input_ty), Type::Graph(_, _));
        if graph && !graph_supported {
            return Err("error: hungarianInfo Graph input is not supported".into());
        }
        let (name, params, call_args) = if graph {
            (
                format!("{base}_graph"),
                vec![pointer.into(), pointer.into()],
                vec![self.emit_expr(&args[0])?, output.into()],
            )
        } else {
            let (data, length) =
                self.emit_path_array_argument(&args[0], &input_ty, "matching.info.input")?;
            (
                base.to_string(),
                vec![pointer.into(), self.usize_type().into(), pointer.into()],
                vec![data, length, output.into()],
            )
        };
        let function = self.declare_libc_fn(&name, &params, None);
        let values: Vec<BasicMetadataValueEnum<'ctx>> =
            call_args.into_iter().map(Into::into).collect();
        self.builder
            .build_call(function, &values, "matching.info.call")
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(self.llvm_type(&result_ty), output, "matching.info.result")
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    pub(super) fn emit_mst_algorithm_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let edge_ty = Type::Array(Box::new(Type::Tuple(vec![
            Type::Usize,
            Type::Usize,
            Type::I32,
        ])));
        let input_ty = self
            .expr_type_hint(&args[0])
            .ok_or("error: cannot infer MST input type")?;
        let graph_input = matches!(transparent_codegen_type(&input_ty), Type::Graph(_, _));
        let result_ty = match target {
            tyrus_core::algorithm::KRUSKAL_INFO_TARGET => Type::Named("KruskalInfo".to_string()),
            tyrus_core::algorithm::PRIM_INFO_TARGET => Type::Named("PrimInfo".to_string()),
            tyrus_core::algorithm::BORUVKA_INFO_TARGET => Type::Named("BoruvkaInfo".to_string()),
            tyrus_core::algorithm::STEINER_TREE_INFO_TARGET => {
                Type::Named("SteinerTreeInfo".to_string())
            }
            _ => edge_ty.clone(),
        };
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "mst.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let edge_stride =
            self.type_size(&Type::Tuple(vec![Type::Usize, Type::Usize, Type::I32]))?;
        let (runtime_name, params, call_args) = match target {
            tyrus_core::algorithm::KRUSKAL_TARGET
            | tyrus_core::algorithm::KRUSKAL_INFO_TARGET
            | tyrus_core::algorithm::BORUVKA_TARGET
            | tyrus_core::algorithm::BORUVKA_INFO_TARGET => {
                if args.len() != 2 {
                    return Err("error: MST edge algorithm expects 2 arguments".into());
                }
                let vertices = self.emit_expr_as(&args[1], &Type::Usize)?;
                let name = match target {
                    tyrus_core::algorithm::KRUSKAL_TARGET => "tyrus_algorithm_kruskal",
                    tyrus_core::algorithm::KRUSKAL_INFO_TARGET => "tyrus_algorithm_kruskal_info",
                    tyrus_core::algorithm::BORUVKA_TARGET => "tyrus_algorithm_boruvka",
                    _ => "tyrus_algorithm_boruvka_info",
                };
                if graph_input {
                    let graph = self.emit_expr_as(&args[0], &input_ty)?;
                    (
                        format!("{name}_graph"),
                        vec![pointer.into(), self.usize_type().into(), pointer.into()],
                        vec![graph, vertices, output.into()],
                    )
                } else {
                    let (data, length) =
                        self.emit_path_array_argument(&args[0], &edge_ty, "mst.edges")?;
                    (
                        name.to_string(),
                        vec![
                            pointer.into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            pointer.into(),
                        ],
                        vec![data, length, edge_stride.into(), vertices, output.into()],
                    )
                }
            }
            tyrus_core::algorithm::PRIM_TARGET | tyrus_core::algorithm::PRIM_INFO_TARGET => {
                if args.len() != 2 {
                    return Err("error: `prim` expects 2 arguments".into());
                }
                let weighted_ty = Type::Array(Box::new(Type::Array(Box::new(Type::Tuple(vec![
                    Type::Usize,
                    Type::I32,
                ])))));
                let origin = self.emit_expr_as(&args[1], &Type::Usize)?;
                let name = if target == tyrus_core::algorithm::PRIM_TARGET {
                    "tyrus_algorithm_prim"
                } else {
                    "tyrus_algorithm_prim_info"
                };
                if graph_input {
                    let graph = self.emit_expr_as(&args[0], &input_ty)?;
                    (
                        format!("{name}_graph"),
                        vec![pointer.into(), self.usize_type().into(), pointer.into()],
                        vec![graph, origin, output.into()],
                    )
                } else {
                    let (data, length) =
                        self.emit_path_array_argument(&args[0], &weighted_ty, "mst.adjacency")?;
                    (
                        name.to_string(),
                        vec![
                            pointer.into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            pointer.into(),
                        ],
                        vec![
                            data,
                            length,
                            self.type_size(&Type::Tuple(vec![Type::Usize, Type::I32]))?
                                .into(),
                            origin,
                            output.into(),
                        ],
                    )
                }
            }
            tyrus_core::algorithm::STEINER_TREE_TARGET
            | tyrus_core::algorithm::STEINER_TREE_INFO_TARGET => {
                if args.len() != 3 {
                    return Err("error: `steinerTree` expects 3 arguments".into());
                }
                let vertices = self.emit_expr_as(&args[1], &Type::Usize)?;
                let terminal_ty = Type::Array(Box::new(Type::Usize));
                let (terminals, terminal_count) =
                    self.emit_path_array_argument(&args[2], &terminal_ty, "mst.terminals")?;
                let name = if target == tyrus_core::algorithm::STEINER_TREE_TARGET {
                    "tyrus_algorithm_steiner_tree"
                } else {
                    "tyrus_algorithm_steiner_tree_info"
                };
                if graph_input {
                    let graph = self.emit_expr_as(&args[0], &input_ty)?;
                    (
                        format!("{name}_graph"),
                        vec![
                            pointer.into(),
                            self.usize_type().into(),
                            pointer.into(),
                            self.usize_type().into(),
                            pointer.into(),
                        ],
                        vec![graph, vertices, terminals, terminal_count, output.into()],
                    )
                } else {
                    let (data, length) =
                        self.emit_path_array_argument(&args[0], &edge_ty, "mst.edges")?;
                    (
                        name.to_string(),
                        vec![
                            pointer.into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            self.usize_type().into(),
                            pointer.into(),
                            self.usize_type().into(),
                            pointer.into(),
                        ],
                        vec![
                            data,
                            length,
                            edge_stride.into(),
                            vertices,
                            terminals,
                            terminal_count,
                            output.into(),
                        ],
                    )
                }
            }
            _ => return Err("error: unknown MST algorithm".into()),
        };
        let function = self.declare_libc_fn(&runtime_name, &params, None);
        let metadata_args: Vec<BasicMetadataValueEnum<'ctx>> =
            call_args.into_iter().map(Into::into).collect();
        self.builder
            .build_call(function, &metadata_args, "mst.call")
            .map_err(|e| e.to_string())?;
        let value = self
            .builder
            .build_load(self.llvm_type(&result_ty), output, "mst.result")
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(value, result_ty);
        Ok(value)
    }

    pub(super) fn emit_flow_algorithm_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let matrix_ty = Type::Array(Box::new(Type::Array(Box::new(Type::I32))));
        let pair_ty = Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize])));
        let triple_ty = Type::Array(Box::new(Type::Tuple(vec![
            Type::Usize,
            Type::Usize,
            Type::I32,
        ])));
        let is_cost = target == tyrus_core::algorithm::MIN_COST_MAX_FLOW_TARGET;
        let is_cut = target == tyrus_core::algorithm::MIN_CUT_TARGET;
        let result_ty = if is_cost {
            Type::Tuple(vec![Type::I32, Type::I32])
        } else if is_cut {
            Type::Tuple(vec![Type::I32, pair_ty.clone()])
        } else {
            Type::Tuple(vec![Type::I32, triple_ty.clone()])
        };
        let value = self
            .builder
            .build_alloca(self.llvm_type(&Type::I32), "flow.value")
            .map_err(|e| e.to_string())?;
        let cost = if is_cost {
            Some(
                self.builder
                    .build_alloca(self.llvm_type(&Type::I32), "flow.cost")
                    .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        let output = if is_cost {
            None
        } else {
            Some(
                self.builder
                    .build_alloca(
                        self.llvm_type(if is_cut { &pair_ty } else { &triple_ty }),
                        "flow.edges",
                    )
                    .map_err(|e| e.to_string())?,
            )
        };
        let (name, params, call_args) = if is_cost {
            let edge_input = Type::Array(Box::new(Type::Tuple(vec![
                Type::Usize,
                Type::Usize,
                Type::I32,
                Type::I32,
            ])));
            let (data, length) =
                self.emit_path_array_argument(&args[0], &edge_input, "flow.edges.input")?;
            (
                "tyrus_algorithm_min_cost_max_flow",
                vec![
                    pointer.into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    pointer.into(),
                    pointer.into(),
                ],
                vec![
                    data,
                    length,
                    self.type_size(&Type::Tuple(vec![
                        Type::Usize,
                        Type::Usize,
                        Type::I32,
                        Type::I32,
                    ]))?
                    .into(),
                    self.emit_expr_as(&args[1], &Type::Usize)?,
                    self.emit_expr_as(&args[2], &Type::Usize)?,
                    self.emit_expr_as(&args[3], &Type::Usize)?,
                    value.into(),
                    cost.unwrap().into(),
                ],
            )
        } else {
            let (data, length) =
                self.emit_path_array_argument(&args[0], &matrix_ty, "flow.matrix")?;
            let runtime = match target {
                tyrus_core::algorithm::EDMONDS_KARP_TARGET => "tyrus_algorithm_edmonds_karp",
                tyrus_core::algorithm::DINIC_TARGET => "tyrus_algorithm_dinic",
                tyrus_core::algorithm::PUSH_RELABEL_TARGET => "tyrus_algorithm_push_relabel",
                _ => "tyrus_algorithm_min_cut",
            };
            (
                runtime,
                vec![
                    pointer.into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    pointer.into(),
                    pointer.into(),
                ],
                vec![
                    data,
                    length,
                    self.emit_expr_as(&args[1], &Type::Usize)?,
                    self.emit_expr_as(&args[2], &Type::Usize)?,
                    value.into(),
                    output.unwrap().into(),
                ],
            )
        };
        let function = self.declare_libc_fn(name, &params, None);
        let metadata: Vec<BasicMetadataValueEnum<'ctx>> =
            call_args.into_iter().map(Into::into).collect();
        self.builder
            .build_call(function, &metadata, "flow.call")
            .map_err(|e| e.to_string())?;
        let first = self
            .builder
            .build_load(self.llvm_type(&Type::I32), value, "flow.value.result")
            .map_err(|e| e.to_string())?;
        let tuple = self.llvm_type(&result_ty).into_struct_type().const_zero();
        let tuple = self
            .builder
            .build_insert_value(tuple, first, 0, "flow.result.value")
            .map_err(|e| e.to_string())?;
        let result = if is_cost {
            let second = self
                .builder
                .build_load(
                    self.llvm_type(&Type::I32),
                    cost.unwrap(),
                    "flow.cost.result",
                )
                .map_err(|e| e.to_string())?;
            self.builder
                .build_insert_value(tuple, second, 1, "flow.result.cost")
                .map_err(|e| e.to_string())?
                .into_struct_value()
        } else {
            let edge_value = self
                .builder
                .build_load(
                    self.llvm_type(if is_cut { &pair_ty } else { &triple_ty }),
                    output.unwrap(),
                    "flow.edges.result",
                )
                .map_err(|e| e.to_string())?;
            self.builder
                .build_insert_value(tuple, edge_value, 1, "flow.result.edges")
                .map_err(|e| e.to_string())?
                .into_struct_value()
        };
        let result = result.into();
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    pub(super) fn emit_flow_info_algorithm_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let (runtime_name, result_ty) = match target {
            tyrus_core::algorithm::EDMONDS_KARP_INFO_TARGET => (
                "tyrus_algorithm_edmonds_karp_info",
                Type::Named("EdmondsKarpInfo".to_string()),
            ),
            tyrus_core::algorithm::DINIC_INFO_TARGET => (
                "tyrus_algorithm_dinic_info",
                Type::Named("DinicInfo".to_string()),
            ),
            tyrus_core::algorithm::PUSH_RELABEL_INFO_TARGET => (
                "tyrus_algorithm_push_relabel_info",
                Type::Named("PushRelabelInfo".to_string()),
            ),
            tyrus_core::algorithm::MIN_CUT_INFO_TARGET => (
                "tyrus_algorithm_min_cut_info",
                Type::Named("MinCutInfo".to_string()),
            ),
            tyrus_core::algorithm::MIN_COST_MAX_FLOW_INFO_TARGET => (
                "tyrus_algorithm_min_cost_max_flow_info",
                Type::Named("MinCostMaxFlowInfo".to_string()),
            ),
            _ => return Err("error: unknown flow info algorithm".into()),
        };
        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "flow.info.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let (params, call_args) = if target == tyrus_core::algorithm::MIN_COST_MAX_FLOW_INFO_TARGET
        {
            let edge_input = Type::Array(Box::new(Type::Tuple(vec![
                Type::Usize,
                Type::Usize,
                Type::I32,
                Type::I32,
            ])));
            let (data, length) =
                self.emit_path_array_argument(&args[0], &edge_input, "flow.info.edges")?;
            (
                vec![
                    pointer.into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    pointer.into(),
                ],
                vec![
                    data,
                    length,
                    self.type_size(&Type::Tuple(vec![
                        Type::Usize,
                        Type::Usize,
                        Type::I32,
                        Type::I32,
                    ]))?
                    .into(),
                    self.emit_expr_as(&args[1], &Type::Usize)?,
                    self.emit_expr_as(&args[2], &Type::Usize)?,
                    self.emit_expr_as(&args[3], &Type::Usize)?,
                    output.into(),
                ],
            )
        } else {
            let matrix_ty = Type::Array(Box::new(Type::Array(Box::new(Type::I32))));
            let (data, length) =
                self.emit_path_array_argument(&args[0], &matrix_ty, "flow.info.matrix")?;
            (
                vec![
                    pointer.into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    pointer.into(),
                ],
                vec![
                    data,
                    length,
                    self.emit_expr_as(&args[1], &Type::Usize)?,
                    self.emit_expr_as(&args[2], &Type::Usize)?,
                    output.into(),
                ],
            )
        };
        let function = self.declare_libc_fn(runtime_name, &params, None);
        let metadata: Vec<BasicMetadataValueEnum<'ctx>> =
            call_args.into_iter().map(Into::into).collect();
        self.builder
            .build_call(function, &metadata, "flow.info.call")
            .map_err(|e| e.to_string())?;
        let result = self
            .builder
            .build_load(self.llvm_type(&result_ty), output, "flow.info.result")
            .map_err(|e| e.to_string())?;
        self.record_owned_native_result(result, result_ty);
        Ok(result)
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_path_algorithm_call(
        &mut self,
        target: &str,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let hinted = self.expr_type_hint(&args[0]);
        let declared = match target {
            tyrus_core::algorithm::IDDFS_TARGET | tyrus_core::algorithm::IDDFS_INFO_TARGET => None,
            tyrus_core::algorithm::FLOYD_WARSHALL_TARGET
            | tyrus_core::algorithm::FLOYD_WARSHALL_INFO_TARGET => Some(Type::Array(Box::new(
                Type::Array(Box::new(Type::Optional(Box::new(Type::I32)))),
            ))),
            _ => Some(Type::Array(Box::new(Type::Array(Box::new(Type::Tuple(
                vec![Type::Usize, Type::I32],
            )))))),
        };
        let input_ty = match (hinted, declared) {
            (Some(hinted), _) if matches!(transparent_codegen_type(&hinted), Type::Graph(_, _)) => {
                hinted
            }
            (_, Some(declared)) => declared,
            (Some(hinted), None) => hinted,
            (None, None) => return Err("error: cannot infer graph path input type".into()),
        };
        let graph_input = matches!(transparent_codegen_type(&input_ty), Type::Graph(_, _));
        let start = if args.len() > 1 {
            Some(self.emit_expr_as(&args[1], &Type::Usize)?)
        } else {
            None
        };
        let end = if args.len() > 2 {
            Some(self.emit_expr_as(&args[2], &Type::Usize)?)
        } else {
            None
        };
        let (runtime_name, result_ty) = match target {
            tyrus_core::algorithm::IDDFS_TARGET => {
                ("tyrus_algorithm_iddfs", Type::Array(Box::new(Type::Usize)))
            }
            tyrus_core::algorithm::IDDFS_INFO_TARGET => (
                "tyrus_algorithm_iddfs_info",
                Type::Named("IddfsInfo".to_string()),
            ),
            tyrus_core::algorithm::DIJKSTRA_TARGET | tyrus_core::algorithm::A_STAR_TARGET => (
                if target == tyrus_core::algorithm::DIJKSTRA_TARGET {
                    "tyrus_algorithm_dijkstra"
                } else {
                    "tyrus_algorithm_a_star"
                },
                Type::Optional(Box::new(Type::Tuple(vec![
                    Type::Array(Box::new(Type::Usize)),
                    Type::I32,
                ]))),
            ),
            tyrus_core::algorithm::DIJKSTRA_INFO_TARGET => (
                "tyrus_algorithm_dijkstra_info",
                Type::Named("DijkstraInfo".to_string()),
            ),
            tyrus_core::algorithm::A_STAR_INFO_TARGET => (
                "tyrus_algorithm_a_star_info",
                Type::Named("AStarInfo".to_string()),
            ),
            tyrus_core::algorithm::BELLMAN_FORD_TARGET | tyrus_core::algorithm::SPFA_TARGET => (
                if target == tyrus_core::algorithm::BELLMAN_FORD_TARGET {
                    "tyrus_algorithm_bellman_ford"
                } else {
                    "tyrus_algorithm_spfa"
                },
                Type::Tuple(vec![
                    Type::Array(Box::new(Type::I32)),
                    Type::Array(Box::new(Type::Optional(Box::new(Type::Usize)))),
                ]),
            ),
            tyrus_core::algorithm::BELLMAN_FORD_INFO_TARGET => (
                "tyrus_algorithm_bellman_ford_info",
                Type::Named("BellmanFordInfo".to_string()),
            ),
            tyrus_core::algorithm::SPFA_INFO_TARGET => (
                "tyrus_algorithm_spfa_info",
                Type::Named("SpfaInfo".to_string()),
            ),
            tyrus_core::algorithm::FLOYD_WARSHALL_TARGET
            | tyrus_core::algorithm::JOHNSON_TARGET => (
                if target == tyrus_core::algorithm::FLOYD_WARSHALL_TARGET {
                    "tyrus_algorithm_floyd_warshall"
                } else {
                    "tyrus_algorithm_johnson"
                },
                Type::Array(Box::new(Type::Array(Box::new(Type::Optional(Box::new(
                    Type::I32,
                )))))),
            ),
            tyrus_core::algorithm::JOHNSON_INFO_TARGET => (
                "tyrus_algorithm_johnson_info",
                Type::Named("JohnsonInfo".to_string()),
            ),
            tyrus_core::algorithm::FLOYD_WARSHALL_INFO_TARGET => (
                "tyrus_algorithm_floyd_warshall_info",
                Type::Named("FloydWarshallInfo".to_string()),
            ),
            _ => return Err("error: unknown graph path algorithm".into()),
        };

        if matches!(
            target,
            tyrus_core::algorithm::DIJKSTRA_INFO_TARGET
                | tyrus_core::algorithm::A_STAR_INFO_TARGET
                | tyrus_core::algorithm::BELLMAN_FORD_INFO_TARGET
                | tyrus_core::algorithm::SPFA_INFO_TARGET
                | tyrus_core::algorithm::JOHNSON_INFO_TARGET
                | tyrus_core::algorithm::FLOYD_WARSHALL_INFO_TARGET
        ) {
            let output = self
                .builder
                .build_alloca(self.llvm_type(&result_ty), "path.study.output")
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(output, self.llvm_type(&result_ty).const_zero())
                .map_err(|error| error.to_string())?;
            let is_a_star = target == tyrus_core::algorithm::A_STAR_INFO_TARGET;
            let is_point_to_point = matches!(
                target,
                tyrus_core::algorithm::DIJKSTRA_INFO_TARGET
                    | tyrus_core::algorithm::A_STAR_INFO_TARGET
            );
            let has_start = matches!(
                target,
                tyrus_core::algorithm::DIJKSTRA_INFO_TARGET
                    | tyrus_core::algorithm::A_STAR_INFO_TARGET
                    | tyrus_core::algorithm::BELLMAN_FORD_INFO_TARGET
                    | tyrus_core::algorithm::SPFA_INFO_TARGET
            );
            let is_matrix = target == tyrus_core::algorithm::FLOYD_WARSHALL_INFO_TARGET;
            let mut call_args: Vec<BasicValueEnum<'ctx>> = Vec::new();
            let mut params: Vec<inkwell::types::BasicTypeEnum<'ctx>> = Vec::new();
            if graph_input {
                call_args.push(self.emit_expr(&args[0])?);
                params.push(pointer.into());
            } else {
                let (data, length) =
                    self.emit_path_array_argument(&args[0], &input_ty, "path.study.input")?;
                call_args.extend([data, length]);
                params.push(pointer.into());
                params.push(self.usize_type().into());
                if !is_matrix {
                    call_args.push(
                        self.type_size(&Type::Tuple(vec![Type::Usize, Type::I32]))?
                            .into(),
                    );
                    params.push(self.usize_type().into());
                }
            }
            if has_start {
                call_args.push(start.ok_or("error: missing source vertex")?);
                params.push(self.usize_type().into());
            }
            if is_point_to_point {
                call_args.push(end.ok_or("error: missing destination vertex")?);
                params.push(self.usize_type().into());
            }
            if is_a_star {
                let heuristic_ty = Type::Array(Box::new(Type::I32));
                let (data, length) =
                    self.emit_path_array_argument(&args[3], &heuristic_ty, "path.study.heuristic")?;
                call_args.extend([data, length]);
                params.push(pointer.into());
                params.push(self.usize_type().into());
            }
            call_args.push(output.into());
            params.push(pointer.into());
            let function_name = if graph_input {
                format!("{runtime_name}_graph")
            } else {
                runtime_name.to_string()
            };
            let function = self.declare_libc_fn(&function_name, &params, None);
            let metadata_args: Vec<BasicMetadataValueEnum<'ctx>> =
                call_args.into_iter().map(Into::into).collect();
            self.builder
                .build_call(function, &metadata_args, "path.study")
                .map_err(|error| error.to_string())?;
            let value = self
                .builder
                .build_load(self.llvm_type(&result_ty), output, "path.study.result")
                .map_err(|error| error.to_string())?;
            if matches!(
                target,
                tyrus_core::algorithm::SPFA_INFO_TARGET
                    | tyrus_core::algorithm::JOHNSON_INFO_TARGET
            ) {
                return self.emit_algorithm_result(result_ty, value, "path.study");
            }
            self.record_owned_native_result(value, result_ty);
            return Ok(value);
        }

        if matches!(
            target,
            tyrus_core::algorithm::IDDFS_TARGET | tyrus_core::algorithm::IDDFS_INFO_TARGET
        ) {
            let output = self
                .builder
                .build_alloca(self.llvm_type(&result_ty), "path.iddfs.output")
                .map_err(|e| e.to_string())?;
            self.builder
                .build_store(output, self.llvm_type(&result_ty).const_zero())
                .map_err(|e| e.to_string())?;
            let call_args = if graph_input {
                let graph = self.emit_expr(&args[0])?;
                vec![
                    graph,
                    start.unwrap(),
                    self.emit_expr_as(&args[2], &Type::Usize)?,
                    output.into(),
                ]
            } else {
                let (data, length) =
                    self.emit_path_array_argument(&args[0], &input_ty, "path.adj")?;
                vec![
                    data,
                    length,
                    start.unwrap(),
                    self.emit_expr_as(&args[2], &Type::Usize)?,
                    output.into(),
                ]
            };
            let fn_name = if graph_input {
                format!("{runtime_name}_graph")
            } else {
                runtime_name.to_string()
            };
            let params = if graph_input {
                vec![
                    pointer.into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    pointer.into(),
                ]
            } else {
                vec![
                    pointer.into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    pointer.into(),
                ]
            };
            let fn_val = self.declare_libc_fn(&fn_name, &params, None);
            let metadata_args: Vec<BasicMetadataValueEnum<'ctx>> =
                call_args.into_iter().map(Into::into).collect();
            self.builder
                .build_call(fn_val, &metadata_args, "path.iddfs")
                .map_err(|e| e.to_string())?;
            let value = self
                .builder
                .build_load(self.llvm_type(&result_ty), output, "path.iddfs.result")
                .map_err(|e| e.to_string())?;
            self.record_owned_native_result(value, result_ty);
            return Ok(value);
        }

        if matches!(
            target,
            tyrus_core::algorithm::DIJKSTRA_TARGET | tyrus_core::algorithm::A_STAR_TARGET
        ) {
            let call_result = if graph_input {
                let graph = self.emit_expr(&args[0])?;
                let mut call_args = vec![graph, start.unwrap(), end.unwrap()];
                let params = if target == tyrus_core::algorithm::A_STAR_TARGET {
                    let h_ty = Type::Array(Box::new(Type::I32));
                    let (h_data, h_len) =
                        self.emit_path_array_argument(&args[3], &h_ty, "path.heuristic")?;
                    call_args.push(h_data);
                    call_args.push(h_len);
                    vec![
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                        pointer.into(),
                        self.usize_type().into(),
                    ]
                } else {
                    vec![
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                    ]
                };
                let name = if target == tyrus_core::algorithm::A_STAR_TARGET {
                    "tyrus_algorithm_a_star_graph"
                } else {
                    "tyrus_algorithm_dijkstra_graph"
                };
                let fn_val = self.declare_libc_fn(name, &params, Some(pointer.into()));
                let metadata_args: Vec<BasicMetadataValueEnum<'ctx>> =
                    call_args.into_iter().map(Into::into).collect();
                self.builder
                    .build_call(fn_val, &metadata_args, "path.shortest")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: shortest path did not return a value")?
                    .into_pointer_value()
            } else {
                let (data, length) =
                    self.emit_path_array_argument(&args[0], &input_ty, "path.adj")?;
                let mut call_args = vec![data, length, start.unwrap(), end.unwrap()];
                let params = if target == tyrus_core::algorithm::A_STAR_TARGET {
                    let h_ty = Type::Array(Box::new(Type::I32));
                    let (h_data, h_len) =
                        self.emit_path_array_argument(&args[3], &h_ty, "path.heuristic")?;
                    call_args.insert(
                        2,
                        self.type_size(&Type::Tuple(vec![Type::Usize, Type::I32]))?
                            .into(),
                    );
                    call_args.push(h_data);
                    call_args.push(h_len);
                    vec![
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                        pointer.into(),
                        self.usize_type().into(),
                    ]
                } else {
                    call_args.insert(
                        2,
                        self.type_size(&Type::Tuple(vec![Type::Usize, Type::I32]))?
                            .into(),
                    );
                    vec![
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                    ]
                };
                let name = if target == tyrus_core::algorithm::A_STAR_TARGET {
                    "tyrus_algorithm_a_star"
                } else {
                    "tyrus_algorithm_dijkstra"
                };
                let fn_val = self.declare_libc_fn(name, &params, Some(pointer.into()));
                let metadata_args: Vec<BasicMetadataValueEnum<'ctx>> =
                    call_args.into_iter().map(Into::into).collect();
                self.builder
                    .build_call(fn_val, &metadata_args, "path.shortest")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("error: shortest path did not return a value")?
                    .into_pointer_value()
            };
            return self.emit_path_result_optional(call_result, "path.shortest.result");
        }

        if matches!(
            target,
            tyrus_core::algorithm::BELLMAN_FORD_TARGET | tyrus_core::algorithm::SPFA_TARGET
        ) {
            let distances_ty = Type::Array(Box::new(Type::I32));
            let previous_ty = Type::Array(Box::new(Type::Optional(Box::new(Type::Usize))));
            let distances = self
                .builder
                .build_alloca(self.llvm_type(&distances_ty), "path.distances.output")
                .map_err(|e| e.to_string())?;
            let previous = self
                .builder
                .build_alloca(self.llvm_type(&previous_ty), "path.previous.output")
                .map_err(|e| e.to_string())?;
            self.builder
                .build_store(distances, self.llvm_type(&distances_ty).const_zero())
                .map_err(|e| e.to_string())?;
            self.builder
                .build_store(previous, self.llvm_type(&previous_ty).const_zero())
                .map_err(|e| e.to_string())?;
            let mut call_args = Vec::new();
            let (params, fn_name) = if graph_input {
                call_args.extend([
                    self.emit_expr(&args[0])?,
                    start.ok_or("error: missing graph source vertex")?,
                    distances.into(),
                    previous.into(),
                ]);
                (
                    vec![
                        pointer.into(),
                        self.usize_type().into(),
                        pointer.into(),
                        pointer.into(),
                    ],
                    format!("{runtime_name}_graph"),
                )
            } else {
                let (data, length) =
                    self.emit_path_array_argument(&args[0], &input_ty, "path.adj")?;
                call_args.extend([
                    data,
                    length,
                    self.type_size(&Type::Tuple(vec![Type::Usize, Type::I32]))?
                        .into(),
                    start.ok_or("error: missing source vertex")?,
                    distances.into(),
                    previous.into(),
                ]);
                (
                    vec![
                        pointer.into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                        self.usize_type().into(),
                        pointer.into(),
                        pointer.into(),
                    ],
                    runtime_name.to_string(),
                )
            };
            let metadata_args: Vec<BasicMetadataValueEnum<'ctx>> =
                call_args.into_iter().map(Into::into).collect();
            let fn_val = self.declare_libc_fn(&fn_name, &params, None);
            self.builder
                .build_call(fn_val, &metadata_args, "path.single_source")
                .map_err(|e| e.to_string())?;
            let distance_value = self
                .builder
                .build_load(
                    self.llvm_type(&distances_ty),
                    distances,
                    "path.distances.result",
                )
                .map_err(|e| e.to_string())?;
            let previous_value = self
                .builder
                .build_load(
                    self.llvm_type(&previous_ty),
                    previous,
                    "path.previous.result",
                )
                .map_err(|e| e.to_string())?;
            let tuple = self
                .builder
                .build_insert_value(
                    self.llvm_type(&result_ty).into_struct_type().const_zero(),
                    distance_value,
                    0,
                    "path.single_source.distances",
                )
                .map_err(|e| e.to_string())?;
            let tuple = self
                .builder
                .build_insert_value(tuple, previous_value, 1, "path.single_source.previous")
                .map_err(|e| e.to_string())?
                .into_struct_value();
            let tuple: BasicValueEnum<'ctx> = tuple.into();
            return self.emit_algorithm_result(result_ty, tuple, "path.single_source");
        }

        let output = self
            .builder
            .build_alloca(self.llvm_type(&result_ty), "path.output")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(output, self.llvm_type(&result_ty).const_zero())
            .map_err(|e| e.to_string())?;
        let is_matrix = matches!(target, tyrus_core::algorithm::FLOYD_WARSHALL_TARGET)
            || (graph_input && matches!(target, tyrus_core::algorithm::JOHNSON_TARGET));
        let graph = if graph_input {
            Some(self.emit_expr(&args[0])?)
        } else {
            None
        };
        let mut call_args = Vec::new();
        let (params, fn_name) = if graph_input {
            call_args.push(graph.unwrap());
            (
                vec![pointer.into(), pointer.into()],
                format!("{runtime_name}_graph"),
            )
        } else if is_matrix {
            let matrix_ty = Type::Array(Box::new(Type::Array(Box::new(Type::Optional(Box::new(
                Type::I32,
            ))))));
            let (data, length) =
                self.emit_path_array_argument(&args[0], &matrix_ty, "path.matrix")?;
            call_args.extend([data, length, output.into()]);
            (
                vec![pointer.into(), self.usize_type().into(), pointer.into()],
                runtime_name.to_string(),
            )
        } else {
            let (data, length) = self.emit_path_array_argument(&args[0], &input_ty, "path.adj")?;
            call_args.extend([
                data,
                length,
                self.type_size(&Type::Tuple(vec![Type::Usize, Type::I32]))?
                    .into(),
                output.into(),
            ]);
            (
                vec![
                    pointer.into(),
                    self.usize_type().into(),
                    self.usize_type().into(),
                    pointer.into(),
                ],
                runtime_name.to_string(),
            )
        };
        if graph_input && is_matrix {
            call_args.push(output.into());
        }
        let fn_val = self.declare_libc_fn(&fn_name, &params, None);
        let metadata_args: Vec<BasicMetadataValueEnum<'ctx>> =
            call_args.into_iter().map(Into::into).collect();
        self.builder
            .build_call(fn_val, &metadata_args, "path.algorithm")
            .map_err(|e| e.to_string())?;
        let value = self
            .builder
            .build_load(self.llvm_type(&result_ty), output, "path.result")
            .map_err(|e| e.to_string())?;
        if target == tyrus_core::algorithm::JOHNSON_TARGET {
            return self.emit_algorithm_result(result_ty, value, "path.johnson");
        }
        self.record_owned_native_result(value, result_ty);
        Ok(value)
    }

    pub(super) fn emit_graph_fill_nodes(
        &mut self,
        graph: BasicValueEnum<'ctx>,
        node_ty: &Type,
        runtime_name: &str,
        label: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let array_ty = Type::Array(Box::new(node_ty.clone()));
        let output = self
            .builder
            .build_alloca(self.llvm_type(&array_ty), "graph.fill.output")
            .map_err(|e| e.to_string())?;
        let fn_val = self.declare_libc_fn(runtime_name, &[pointer.into(), pointer.into()], None);
        self.builder
            .build_call(fn_val, &[graph.into(), output.into()], label)
            .map_err(|e| e.to_string())?;
        self.builder
            .build_load(self.llvm_type(&array_ty), output, "graph.fill.result")
            .map_err(|e| e.to_string())
    }

    pub(super) fn emit_graph_fill_weights(
        &mut self,
        handle: PointerValue<'ctx>,
        weight_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let array_ty = Type::Array(Box::new(weight_ty.clone()));
        let output = self
            .builder
            .build_alloca(self.llvm_type(&array_ty), "graph.edges.weights.output")
            .map_err(|e| e.to_string())?;
        let fn_val = self.declare_libc_fn(
            "tyrus_graph_edges_weights",
            &[pointer.into(), pointer.into()],
            None,
        );
        self.builder
            .build_call(
                fn_val,
                &[handle.into(), output.into()],
                "graph.edges.weights",
            )
            .map_err(|e| e.to_string())?;
        self.builder
            .build_load(
                self.llvm_type(&array_ty),
                output,
                "graph.edges.weights.result",
            )
            .map_err(|e| e.to_string())
    }

    pub(super) fn emit_graph_new_handle(
        &mut self,
        node_ty: &Type,
        weight_ty: &Type,
    ) -> Result<PointerValue<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let node_size = self.type_size(node_ty)?;
        let weight_size = self.type_size(weight_ty)?;
        let node_tag = self
            .context
            .i8_type()
            .const_int(graph_type_tag(node_ty), false);
        let weight_tag = self
            .context
            .i8_type()
            .const_int(graph_type_tag(weight_ty), false);
        let fn_val = self.declare_libc_fn(
            "tyrus_graph_new",
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                self.context.i8_type().into(),
                self.context.i8_type().into(),
            ],
            Some(pointer.into()),
        );
        let result = self
            .builder
            .build_call(
                fn_val,
                &[
                    node_size.into(),
                    weight_size.into(),
                    node_tag.into(),
                    weight_tag.into(),
                ],
                "graph.map.new",
            )
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        Ok(result)
    }

    pub(super) fn emit_graph_array_element(
        &self,
        data: PointerValue<'ctx>,
        element_ty: &Type,
        index: IntValue<'ctx>,
        name: &str,
    ) -> BasicValueEnum<'ctx> {
        let llvm_element = self.llvm_type(element_ty);
        let slot = unsafe {
            self.builder
                .build_gep(llvm_element, data, &[index], name)
                .unwrap()
        };
        self.builder.build_load(llvm_element, slot, name).unwrap()
    }

    pub(super) fn emit_graph_free_collection_array(
        &mut self,
        value: BasicValueEnum<'ctx>,
        element_ty: &Type,
    ) {
        if matches!(transparent_codegen_type(element_ty), Type::Str) {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let array_ty = Type::Array(Box::new(element_ty.clone()));
            let slot = self
                .builder
                .build_alloca(self.llvm_type(&array_ty), "graph.intermediate.array")
                .unwrap();
            self.builder.build_store(slot, value).unwrap();
            let drop_contents =
                self.declare_libc_fn("tyrus_array_drop_string_contents", &[pointer.into()], None);
            self.builder
                .build_call(drop_contents, &[slot.into()], "")
                .unwrap();
        } else {
            self.emit_owned_native_buffer_free(value);
        }
    }

    pub(super) fn emit_graph_edges(
        &mut self,
        handle: PointerValue<'ctx>,
        node_ty: &Type,
        weight_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let sources = self.emit_graph_fill_nodes(
            handle.into(),
            node_ty,
            "tyrus_graph_edges_sources",
            "graph.edges.sources",
        )?;
        let targets = self.emit_graph_fill_nodes(
            handle.into(),
            node_ty,
            "tyrus_graph_edges_targets",
            "graph.edges.targets",
        )?;
        let weights = self.emit_graph_fill_weights(handle, weight_ty)?;
        let sources_struct = sources.into_struct_value();
        let length = self
            .builder
            .build_extract_value(sources_struct, 1, "graph.edges.length")
            .unwrap()
            .into_int_value();
        let source_data = self
            .builder
            .build_extract_value(sources_struct, 0, "graph.edges.source_data")
            .unwrap()
            .into_pointer_value();
        let target_data = self
            .builder
            .build_extract_value(targets.into_struct_value(), 0, "graph.edges.target_data")
            .unwrap()
            .into_pointer_value();
        let weight_data = self
            .builder
            .build_extract_value(weights.into_struct_value(), 0, "graph.edges.weight_data")
            .unwrap()
            .into_pointer_value();
        let tuple_ty = Type::Tuple(vec![node_ty.clone(), node_ty.clone(), weight_ty.clone()]);
        let tuple_llvm = self.llvm_type(&tuple_ty);
        let entry_size = tuple_llvm
            .size_of()
            .ok_or_else(|| "error: graph edge tuple type has no native size".to_string())?;
        let bytes = self
            .builder
            .build_int_mul(entry_size, length, "graph.edges.bytes")
            .unwrap();
        let malloc =
            self.declare_libc_fn("malloc", &[self.usize_type().into()], Some(pointer.into()));
        let data = self
            .builder
            .build_call(malloc, &[bytes.into()], "graph.edges.data")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        self.emit_forward_loop(length, "graph.edges", |this, index| {
            let source =
                this.emit_graph_array_element(source_data, node_ty, index, "graph.edges.source");
            let target =
                this.emit_graph_array_element(target_data, node_ty, index, "graph.edges.target");
            let weight =
                this.emit_graph_array_element(weight_data, weight_ty, index, "graph.edges.weight");
            let mut tuple = tuple_llvm.into_struct_type().get_undef();
            tuple = this
                .builder
                .build_insert_value(tuple, source, 0, "graph.edges.tuple.s")
                .unwrap()
                .into_struct_value();
            tuple = this
                .builder
                .build_insert_value(tuple, target, 1, "graph.edges.tuple.t")
                .unwrap()
                .into_struct_value();
            tuple = this
                .builder
                .build_insert_value(tuple, weight, 2, "graph.edges.tuple.w")
                .unwrap()
                .into_struct_value();
            let dest = unsafe {
                this.builder
                    .build_gep(tuple_llvm, data, &[index], "graph.edges.dest")
                    .unwrap()
            };
            this.builder.build_store(dest, tuple).unwrap();
            Ok(())
        })?;
        self.emit_owned_native_buffer_free(sources);
        self.emit_owned_native_buffer_free(targets);
        self.emit_owned_native_buffer_free(weights);
        let array_ty = self.llvm_type(&Type::Array(Box::new(tuple_ty.clone())));
        let mut result = array_ty.into_struct_type().get_undef();
        result = self
            .builder
            .build_insert_value(result, data, 0, "graph.edges.arr_data")
            .unwrap()
            .into_struct_value();
        result = self
            .builder
            .build_insert_value(result, length, 1, "graph.edges.arr_len")
            .unwrap()
            .into_struct_value();
        result = self
            .builder
            .build_insert_value(result, length, 2, "graph.edges.arr_cap")
            .unwrap()
            .into_struct_value();
        let result = result.into();
        self.record_owned_native_result(result, Type::Array(Box::new(tuple_ty)));
        Ok(result)
    }

    pub(super) fn emit_graph_map_edges(
        &mut self,
        handle: PointerValue<'ctx>,
        node_ty: &Type,
        weight_ty: &Type,
        new_weight_ty: &Type,
        callback: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let new_graph = self.emit_graph_new_handle(node_ty, new_weight_ty)?;
        let new_graph_ty = Type::Graph(Box::new(node_ty.clone()), Box::new(new_weight_ty.clone()));
        self.record_owned_native_result(new_graph.into(), new_graph_ty);
        let nodes = self.emit_graph_fill_nodes(
            handle.into(),
            node_ty,
            "tyrus_graph_nodes",
            "graph.map_edges.nodes",
        )?;
        let nodes_struct = nodes.into_struct_value();
        let node_count = self
            .builder
            .build_extract_value(nodes_struct, 1, "graph.map_edges.node_count")
            .unwrap()
            .into_int_value();
        let node_data = self
            .builder
            .build_extract_value(nodes_struct, 0, "graph.map_edges.node_data")
            .unwrap()
            .into_pointer_value();
        self.emit_forward_loop(node_count, "graph.map_edges.copy_nodes", |this, index| {
            let node =
                this.emit_graph_array_element(node_data, node_ty, index, "graph.map_edges.node");
            let slot = this.emit_value_slot(node, node_ty);
            let add_node = this.declare_libc_fn(
                "tyrus_graph_add_node",
                &[pointer.into(), pointer.into()],
                None,
            );
            this.builder
                .build_call(
                    add_node,
                    &[new_graph.into(), slot.into()],
                    "graph.map_edges.copy_node",
                )
                .unwrap();
            Ok(())
        })?;
        let sources = self.emit_graph_fill_nodes(
            handle.into(),
            node_ty,
            "tyrus_graph_edges_sources",
            "graph.map_edges.sources",
        )?;
        let targets = self.emit_graph_fill_nodes(
            handle.into(),
            node_ty,
            "tyrus_graph_edges_targets",
            "graph.map_edges.targets",
        )?;
        let weights = self.emit_graph_fill_weights(handle, weight_ty)?;
        let sources_struct = sources.into_struct_value();
        let edge_count = self
            .builder
            .build_extract_value(sources_struct, 1, "graph.map_edges.edge_count")
            .unwrap()
            .into_int_value();
        let source_data = self
            .builder
            .build_extract_value(sources_struct, 0, "graph.map_edges.source_data")
            .unwrap()
            .into_pointer_value();
        let target_data = self
            .builder
            .build_extract_value(
                targets.into_struct_value(),
                0,
                "graph.map_edges.target_data",
            )
            .unwrap()
            .into_pointer_value();
        let weight_data = self
            .builder
            .build_extract_value(
                weights.into_struct_value(),
                0,
                "graph.map_edges.weight_data",
            )
            .unwrap()
            .into_pointer_value();
        let (closure, closure_params) =
            self.emit_callback(callback, std::slice::from_ref(weight_ty), new_weight_ty)?;
        self.emit_forward_loop(edge_count, "graph.map_edges", |this, index| {
            let source = this.emit_graph_array_element(
                source_data,
                node_ty,
                index,
                "graph.map_edges.source",
            );
            let target = this.emit_graph_array_element(
                target_data,
                node_ty,
                index,
                "graph.map_edges.target",
            );
            let weight = this.emit_graph_array_element(
                weight_data,
                weight_ty,
                index,
                "graph.map_edges.weight",
            );
            let mapped = this
                .emit_closure_call(closure, &closure_params, new_weight_ty, &[weight])?
                .ok_or("internal: mapEdges callback returned void")?;
            let source_slot = this.emit_value_slot(source, node_ty);
            let target_slot = this.emit_value_slot(target, node_ty);
            let mapped_slot = this.emit_value_slot(mapped, new_weight_ty);
            let add_edge = this.declare_libc_fn(
                "tyrus_graph_add_edge",
                &[
                    pointer.into(),
                    pointer.into(),
                    pointer.into(),
                    pointer.into(),
                ],
                None,
            );
            this.builder
                .build_call(
                    add_edge,
                    &[
                        new_graph.into(),
                        source_slot.into(),
                        target_slot.into(),
                        mapped_slot.into(),
                    ],
                    "graph.map_edges.add_edge",
                )
                .unwrap();
            Ok(())
        })?;
        self.release_synchronous_closure(closure, false)?;
        self.emit_graph_free_collection_array(nodes, node_ty);
        self.emit_graph_free_collection_array(sources, node_ty);
        self.emit_graph_free_collection_array(targets, node_ty);
        self.emit_graph_free_collection_array(weights, weight_ty);
        Ok(new_graph.into())
    }

    pub(super) fn emit_graph_map_nodes(
        &mut self,
        handle: PointerValue<'ctx>,
        node_ty: &Type,
        weight_ty: &Type,
        new_node_ty: &Type,
        callback: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let new_graph = self.emit_graph_new_handle(new_node_ty, weight_ty)?;
        let new_graph_ty = Type::Graph(Box::new(new_node_ty.clone()), Box::new(weight_ty.clone()));
        self.record_owned_native_result(new_graph.into(), new_graph_ty);
        let nodes = self.emit_graph_fill_nodes(
            handle.into(),
            node_ty,
            "tyrus_graph_nodes",
            "graph.map_nodes.nodes",
        )?;
        let nodes_struct = nodes.into_struct_value();
        let node_count = self
            .builder
            .build_extract_value(nodes_struct, 1, "graph.map_nodes.node_count")
            .unwrap()
            .into_int_value();
        let node_data = self
            .builder
            .build_extract_value(nodes_struct, 0, "graph.map_nodes.node_data")
            .unwrap()
            .into_pointer_value();
        let (closure, closure_params) =
            self.emit_callback(callback, std::slice::from_ref(node_ty), new_node_ty)?;
        self.emit_forward_loop(node_count, "graph.map_nodes.add", |this, index| {
            let node =
                this.emit_graph_array_element(node_data, node_ty, index, "graph.map_nodes.node");
            let mapped = this
                .emit_closure_call(closure, &closure_params, new_node_ty, &[node])?
                .ok_or("internal: mapNodes callback returned void")?;
            let slot = this.emit_value_slot(mapped, new_node_ty);
            let add_node = this.declare_libc_fn(
                "tyrus_graph_add_node",
                &[pointer.into(), pointer.into()],
                None,
            );
            this.builder
                .build_call(
                    add_node,
                    &[new_graph.into(), slot.into()],
                    "graph.map_nodes.add_node",
                )
                .unwrap();
            if matches!(transparent_codegen_type(new_node_ty), Type::Str) {
                this.emit_string_release(mapped.into_struct_value())?;
            }
            Ok(())
        })?;
        let sources = self.emit_graph_fill_nodes(
            handle.into(),
            node_ty,
            "tyrus_graph_edges_sources",
            "graph.map_nodes.sources",
        )?;
        let targets = self.emit_graph_fill_nodes(
            handle.into(),
            node_ty,
            "tyrus_graph_edges_targets",
            "graph.map_nodes.targets",
        )?;
        let weights = self.emit_graph_fill_weights(handle, weight_ty)?;
        let sources_struct = sources.into_struct_value();
        let edge_count = self
            .builder
            .build_extract_value(sources_struct, 1, "graph.map_nodes.edge_count")
            .unwrap()
            .into_int_value();
        let source_data = self
            .builder
            .build_extract_value(sources_struct, 0, "graph.map_nodes.source_data")
            .unwrap()
            .into_pointer_value();
        let target_data = self
            .builder
            .build_extract_value(
                targets.into_struct_value(),
                0,
                "graph.map_nodes.target_data",
            )
            .unwrap()
            .into_pointer_value();
        let weight_data = self
            .builder
            .build_extract_value(
                weights.into_struct_value(),
                0,
                "graph.map_nodes.weight_data",
            )
            .unwrap()
            .into_pointer_value();
        self.emit_forward_loop(edge_count, "graph.map_nodes.edges", |this, index| {
            let source = this.emit_graph_array_element(
                source_data,
                node_ty,
                index,
                "graph.map_nodes.source",
            );
            let target = this.emit_graph_array_element(
                target_data,
                node_ty,
                index,
                "graph.map_nodes.target",
            );
            let weight = this.emit_graph_array_element(
                weight_data,
                weight_ty,
                index,
                "graph.map_nodes.weight",
            );
            let mapped_source = this
                .emit_closure_call(closure, &closure_params, new_node_ty, &[source])?
                .ok_or("internal: mapNodes callback returned void")?;
            let mapped_target = this
                .emit_closure_call(closure, &closure_params, new_node_ty, &[target])?
                .ok_or("internal: mapNodes callback returned void")?;
            let source_slot = this.emit_value_slot(mapped_source, new_node_ty);
            let target_slot = this.emit_value_slot(mapped_target, new_node_ty);
            let weight_slot = this.emit_value_slot(weight, weight_ty);
            let add_edge = this.declare_libc_fn(
                "tyrus_graph_add_edge",
                &[
                    pointer.into(),
                    pointer.into(),
                    pointer.into(),
                    pointer.into(),
                ],
                None,
            );
            this.builder
                .build_call(
                    add_edge,
                    &[
                        new_graph.into(),
                        source_slot.into(),
                        target_slot.into(),
                        weight_slot.into(),
                    ],
                    "graph.map_nodes.add_edge",
                )
                .unwrap();
            if matches!(transparent_codegen_type(new_node_ty), Type::Str) {
                this.emit_string_release(mapped_source.into_struct_value())?;
                this.emit_string_release(mapped_target.into_struct_value())?;
            }
            Ok(())
        })?;
        self.release_synchronous_closure(closure, false)?;
        self.emit_graph_free_collection_array(nodes, node_ty);
        self.emit_graph_free_collection_array(sources, node_ty);
        self.emit_graph_free_collection_array(targets, node_ty);
        self.emit_graph_free_collection_array(weights, weight_ty);
        Ok(new_graph.into())
    }
}
