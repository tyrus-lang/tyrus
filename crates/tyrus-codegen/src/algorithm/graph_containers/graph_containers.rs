use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_graph_container_call(
        &mut self,
        target: &str,
        args: &[Expr],
        element_type: Option<&Type>,
        value_type: Option<&Type>,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let result = match target {
            tyrus_core::algorithm::GRAPH_NEW_TARGET => {
                if !args.is_empty() {
                    return Err("error: `Graph` expects 0 arguments".into());
                }
                let node_ty = element_type.ok_or_else(|| {
                    "error: `Graph` requires type arguments like `Graph<string, number>()`"
                        .to_string()
                })?;
                let weight_ty = value_type.ok_or_else(|| {
                    "error: `Graph` requires a weight type argument like `Graph<string, number>()`"
                        .to_string()
                })?;
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
                self.builder
                    .build_call(
                        fn_val,
                        &[
                            node_size.into(),
                            weight_size.into(),
                            node_tag.into(),
                            weight_tag.into(),
                        ],
                        "graph.new",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::GRAPH_ADD_NODE_TARGET => {
                if args.len() != 2 {
                    return Err("error: `addNode` expects 1 argument".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let node_ty = element_type
                    .ok_or_else(|| "error: cannot infer node type for `addNode`".to_string())?;
                let node = self.emit_expr_as(&args[1], node_ty)?;
                let slot = self.emit_value_slot(node, node_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_graph_add_node",
                    &[pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(fn_val, &[graph.into(), slot.into()], "graph.add_node")
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::GRAPH_ADD_EDGE_TARGET => {
                if args.len() != 4 {
                    return Err("error: `addEdge` expects 3 arguments".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let node_ty = element_type
                    .ok_or_else(|| "error: cannot infer node type for `addEdge`".to_string())?;
                let weight_ty = value_type
                    .ok_or_else(|| "error: cannot infer weight type for `addEdge`".to_string())?;
                let source = self.emit_expr_as(&args[1], node_ty)?;
                let source_slot = self.emit_value_slot(source, node_ty);
                let target = self.emit_expr_as(&args[2], node_ty)?;
                let target_slot = self.emit_value_slot(target, node_ty);
                let weight = self.emit_expr_as(&args[3], weight_ty)?;
                let weight_slot = self.emit_value_slot(weight, weight_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_graph_add_edge",
                    &[
                        pointer.into(),
                        pointer.into(),
                        pointer.into(),
                        pointer.into(),
                    ],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[
                            graph.into(),
                            source_slot.into(),
                            target_slot.into(),
                            weight_slot.into(),
                        ],
                        "graph.add_edge",
                    )
                    .map_err(|e| e.to_string())?;
                self.context.i32_type().const_int(0, false).into()
            }
            tyrus_core::algorithm::GRAPH_REMOVE_NODE_TARGET => {
                if args.len() != 2 {
                    return Err("error: `removeNode` expects 1 argument".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let node_ty = element_type
                    .ok_or_else(|| "error: cannot infer node type for `removeNode`".to_string())?;
                let node = self.emit_expr_as(&args[1], node_ty)?;
                let slot = self.emit_value_slot(node, node_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_graph_remove_node",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[graph.into(), slot.into()], "graph.remove_node")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::GRAPH_REMOVE_EDGE_TARGET => {
                if args.len() != 3 {
                    return Err("error: `removeEdge` expects 2 arguments".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let node_ty = element_type
                    .ok_or_else(|| "error: cannot infer node type for `removeEdge`".to_string())?;
                let source = self.emit_expr_as(&args[1], node_ty)?;
                let source_slot = self.emit_value_slot(source, node_ty);
                let target = self.emit_expr_as(&args[2], node_ty)?;
                let target_slot = self.emit_value_slot(target, node_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_graph_remove_edge",
                    &[pointer.into(), pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[graph.into(), source_slot.into(), target_slot.into()],
                        "graph.remove_edge",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::GRAPH_HAS_NODE_TARGET => {
                if args.len() != 2 {
                    return Err("error: `hasNode` expects 1 argument".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let node_ty = element_type
                    .ok_or_else(|| "error: cannot infer node type for `hasNode`".to_string())?;
                let node = self.emit_expr_as(&args[1], node_ty)?;
                let slot = self.emit_value_slot(node, node_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_graph_has_node",
                    &[pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[graph.into(), slot.into()], "graph.has_node")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::GRAPH_HAS_EDGE_TARGET => {
                if args.len() != 3 {
                    return Err("error: `hasEdge` expects 2 arguments".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let node_ty = element_type
                    .ok_or_else(|| "error: cannot infer node type for `hasEdge`".to_string())?;
                let source = self.emit_expr_as(&args[1], node_ty)?;
                let source_slot = self.emit_value_slot(source, node_ty);
                let target = self.emit_expr_as(&args[2], node_ty)?;
                let target_slot = self.emit_value_slot(target, node_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_graph_has_edge",
                    &[pointer.into(), pointer.into(), pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[graph.into(), source_slot.into(), target_slot.into()],
                        "graph.has_edge",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::GRAPH_EDGE_TARGET => {
                if args.len() != 3 {
                    return Err("error: `edge` expects 2 arguments".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let node_ty = element_type
                    .ok_or_else(|| "error: cannot infer node type for `edge`".to_string())?;
                let source = self.emit_expr_as(&args[1], node_ty)?;
                let source_slot = self.emit_value_slot(source, node_ty);
                let target = self.emit_expr_as(&args[2], node_ty)?;
                let target_slot = self.emit_value_slot(target, node_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_graph_edge",
                    &[pointer.into(), pointer.into(), pointer.into()],
                    Some(pointer.into()),
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[graph.into(), source_slot.into(), target_slot.into()],
                        "graph.edge",
                    )
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::GRAPH_NEIGHBORS_TARGET => {
                if args.len() != 2 {
                    return Err("error: `neighbors` expects 1 argument".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let node_ty = element_type
                    .ok_or_else(|| "error: cannot infer node type for `neighbors`".to_string())?;
                let node = self.emit_expr_as(&args[1], node_ty)?;
                let slot = self.emit_value_slot(node, node_ty);
                let array_ty = Type::Array(Box::new(node_ty.clone()));
                let output = self
                    .builder
                    .build_alloca(self.llvm_type(&array_ty), "graph.neighbors.output")
                    .map_err(|e| e.to_string())?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_graph_neighbors",
                    &[pointer.into(), pointer.into(), pointer.into()],
                    None,
                );
                self.builder
                    .build_call(
                        fn_val,
                        &[graph.into(), slot.into(), output.into()],
                        "graph.neighbors",
                    )
                    .map_err(|e| e.to_string())?;
                let result = self
                    .builder
                    .build_load(self.llvm_type(&array_ty), output, "graph.neighbors.result")
                    .map_err(|e| e.to_string())?;
                self.record_owned_native_result(result, array_ty);
                result
            }
            tyrus_core::algorithm::GRAPH_DEGREE_TARGET => {
                if args.len() != 2 {
                    return Err("error: `degree` expects 1 argument".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let node_ty = element_type
                    .ok_or_else(|| "error: cannot infer node type for `degree`".to_string())?;
                let node = self.emit_expr_as(&args[1], node_ty)?;
                let slot = self.emit_value_slot(node, node_ty);
                let fn_val = self.declare_libc_fn(
                    "tyrus_graph_degree",
                    &[pointer.into(), pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[graph.into(), slot.into()], "graph.degree")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::GRAPH_NODE_COUNT_TARGET
            | tyrus_core::algorithm::GRAPH_EDGE_COUNT_TARGET => {
                if args.len() != 1 {
                    return Err("error: `nodeCount`/`edgeCount` expect 0 arguments".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::GRAPH_NODE_COUNT_TARGET => {
                        ("tyrus_graph_node_count", "graph.node_count")
                    }
                    _ => ("tyrus_graph_edge_count", "graph.edge_count"),
                };
                let fn_val = self.declare_libc_fn(
                    runtime_name,
                    &[pointer.into()],
                    Some(self.usize_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[graph.into()], label)
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::GRAPH_IS_EMPTY_TARGET => {
                if args.len() != 1 {
                    return Err("error: `isEmpty` expects 0 arguments".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_graph_is_empty",
                    &[pointer.into()],
                    Some(self.context.bool_type().into()),
                );
                self.builder
                    .build_call(fn_val, &[graph.into()], "graph.is_empty")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
            }
            tyrus_core::algorithm::GRAPH_NODES_TARGET => {
                if args.len() != 1 {
                    return Err("error: `nodes` expects 0 arguments".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let node_ty = element_type
                    .ok_or_else(|| "error: cannot infer node type for `nodes`".to_string())?;
                let result =
                    self.emit_graph_fill_nodes(graph, node_ty, "tyrus_graph_nodes", "graph.nodes")?;
                self.record_owned_native_result(result, Type::Array(Box::new(node_ty.clone())));
                result
            }
            tyrus_core::algorithm::GRAPH_EDGES_SOURCES_TARGET
            | tyrus_core::algorithm::GRAPH_EDGES_TARGETS_TARGET => {
                if args.len() != 1 {
                    return Err(
                        "error: internal `edgesSources`/`edgesTargets` take the graph".into(),
                    );
                }
                let graph = self.emit_expr(&args[0])?;
                let node_ty = element_type
                    .ok_or_else(|| "error: cannot infer node type for `edges`".to_string())?;
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::GRAPH_EDGES_SOURCES_TARGET => {
                        ("tyrus_graph_edges_sources", "graph.edges.sources")
                    }
                    _ => ("tyrus_graph_edges_targets", "graph.edges.targets"),
                };
                self.emit_graph_fill_nodes(graph, node_ty, runtime_name, label)?
            }
            tyrus_core::algorithm::GRAPH_EDGES_WEIGHTS_TARGET => {
                if args.len() != 1 {
                    return Err("error: internal `edgesWeights` takes the graph".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let weight_ty = value_type
                    .ok_or_else(|| "error: cannot infer weight type for `edges`".to_string())?;
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
                        &[graph.into(), output.into()],
                        "graph.edges.weights",
                    )
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_load(
                        self.llvm_type(&array_ty),
                        output,
                        "graph.edges.weights.result",
                    )
                    .map_err(|e| e.to_string())?
            }
            tyrus_core::algorithm::GRAPH_REVERSE_TARGET => {
                if args.len() != 1 {
                    return Err("error: `reverse` expects 0 arguments".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let fn_val = self.declare_libc_fn(
                    "tyrus_graph_reverse",
                    &[pointer.into()],
                    Some(pointer.into()),
                );
                let result = self
                    .builder
                    .build_call(fn_val, &[graph.into()], "graph.reverse")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(
                    result,
                    Type::Graph(
                        Box::new(element_type.cloned().unwrap_or(Type::Unknown)),
                        Box::new(value_type.cloned().unwrap_or(Type::Unknown)),
                    ),
                );
                result
            }
            tyrus_core::algorithm::GRAPH_TO_DOT_TARGET
            | tyrus_core::algorithm::GRAPH_TO_STRING_TARGET => {
                if args.len() != 1 {
                    return Err("error: `toDot`/`toString` expect 0 arguments".into());
                }
                let graph = self.emit_expr(&args[0])?;
                let (runtime_name, label) = match target {
                    tyrus_core::algorithm::GRAPH_TO_DOT_TARGET => {
                        ("tyrus_graph_to_dot", "graph.to_dot")
                    }
                    _ => ("tyrus_graph_to_string", "graph.to_string"),
                };
                let fn_val = self.declare_libc_fn(
                    runtime_name,
                    &[pointer.into()],
                    Some(self.string_abi_type()),
                );
                let result = self
                    .builder
                    .build_call(fn_val, &[graph.into()], label)
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                self.record_owned_native_result(result, Type::Str);
                result
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }
}
