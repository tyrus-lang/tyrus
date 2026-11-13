use super::*;

#[allow(clippy::too_many_lines)]
pub(crate) fn algorithm_builtins() -> HashMap<String, ExportInfo> {
    let mut builtins = HashMap::default();
    builtins.insert(
        "suffixArray".to_string(),
        ExportInfo {
            target: SUFFIX_ARRAY_TARGET.to_string(),
            ty: Type::Function {
                params: vec![Type::Str],
                ret: Box::new(Type::Array(Box::new(Type::Usize))),
            },
            interface_fields: None,
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "suffixArrayInfo".to_string(),
        ExportInfo {
            target: SUFFIX_ARRAY_INFO_TARGET.to_string(),
            ty: Type::Function {
                params: vec![Type::Str],
                ret: Box::new(Type::Named("SuffixArrayInfo".to_string())),
            },
            interface_fields: None,
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "SuffixArrayInfo".to_string(),
        ExportInfo {
            target: "SuffixArrayInfo".to_string(),
            ty: Type::Named("SuffixArrayInfo".to_string()),
            interface_fields: Some(crate::builtin_interfaces::suffix_array_info_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "suffixTreeUkkonen".to_string(),
        ExportInfo {
            target: SUFFIX_TREE_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![Type::Str],
                ret: Box::new(Type::SuffixTree),
            },
            interface_fields: Some(suffix_tree_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "SuffixTree".to_string(),
        ExportInfo {
            target: "SuffixTree".to_string(),
            ty: Type::Named("SuffixTree".to_string()),
            interface_fields: Some(suffix_tree_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "lcaBinaryLifting".to_string(),
        ExportInfo {
            target: LCA_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Array(Box::new(Type::Usize)))),
                    Type::Usize,
                ],
                ret: Box::new(Type::Lca),
            },
            interface_fields: Some(lca_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "Lca".to_string(),
        ExportInfo {
            target: "Lca".to_string(),
            ty: Type::Named("Lca".to_string()),
            interface_fields: Some(lca_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "heavyLightDecomposition".to_string(),
        ExportInfo {
            target: HLD_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![
                    Type::Array(Box::new(Type::Array(Box::new(Type::Usize)))),
                    Type::Usize,
                ],
                ret: Box::new(Type::Hld),
            },
            interface_fields: Some(hld_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "Hld".to_string(),
        ExportInfo {
            target: "Hld".to_string(),
            ty: Type::Named("Hld".to_string()),
            interface_fields: Some(hld_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "centroidDecomposition".to_string(),
        ExportInfo {
            target: CENTROID_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![Type::Array(Box::new(Type::Array(Box::new(Type::Usize))))],
                ret: Box::new(Type::Centroid),
            },
            interface_fields: Some(centroid_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "Centroid".to_string(),
        ExportInfo {
            target: "Centroid".to_string(),
            ty: Type::Named("Centroid".to_string()),
            interface_fields: Some(centroid_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "BloomFilter".to_string(),
        ExportInfo {
            target: BLOOM_FILTER_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![Type::Usize, Type::Usize],
                ret: Box::new(Type::Named("BloomFilter".to_string())),
            },
            interface_fields: Some(bloom_filter_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "bfs".to_string(),
        path_algorithm_export(
            BFS_TARGET,
            vec![path_input(unweighted_adjacency()), Type::Usize],
            usize_array(),
            false,
        ),
    );
    builtins.insert(
        "dfs".to_string(),
        path_algorithm_export(
            DFS_TARGET,
            vec![path_input(unweighted_adjacency()), Type::Usize],
            usize_array(),
            false,
        ),
    );
    builtins.insert(
        "iddfs".to_string(),
        path_algorithm_export(
            IDDFS_TARGET,
            vec![path_input(unweighted_adjacency()), Type::Usize, Type::Usize],
            usize_array(),
            false,
        ),
    );
    for (name, target, params, result) in [
        (
            "bfsInfo",
            BFS_INFO_TARGET,
            vec![path_input(unweighted_adjacency()), Type::Usize],
            "TraversalInfo",
        ),
        (
            "dfsInfo",
            DFS_INFO_TARGET,
            vec![path_input(unweighted_adjacency()), Type::Usize],
            "TraversalInfo",
        ),
        (
            "iddfsInfo",
            IDDFS_INFO_TARGET,
            vec![path_input(unweighted_adjacency()), Type::Usize, Type::Usize],
            "IddfsInfo",
        ),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(target, params, Type::Named(result.to_string()), false),
        );
    }
    for (name, fields) in [
        (
            "TraversalInfo",
            crate::builtin_interfaces::traversal_info_fields(),
        ),
        ("IddfsInfo", crate::builtin_interfaces::iddfs_info_fields()),
    ] {
        builtins.insert(
            name.to_string(),
            ExportInfo {
                target: name.to_string(),
                ty: Type::Named(name.to_string()),
                interface_fields: Some(fields),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        );
    }
    for (name, target, result) in [
        (
            "findBridgesInfo",
            FIND_BRIDGES_INFO_TARGET,
            "FindBridgesInfo",
        ),
        (
            "findCutVerticesInfo",
            FIND_CUT_VERTICES_INFO_TARGET,
            "FindCutVerticesInfo",
        ),
        (
            "biconnectedComponentsInfo",
            BICONNECTED_COMPONENTS_INFO_TARGET,
            "BiconnectedComponentsInfo",
        ),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(
                target,
                vec![path_input(unweighted_adjacency())],
                Type::Named(result.to_string()),
                false,
            ),
        );
    }
    for (name, fields) in [
        (
            "ConnectivityInfo",
            crate::builtin_interfaces::connectivity_info_fields(),
        ),
        (
            "FindBridgesInfo",
            crate::builtin_interfaces::find_bridges_info_fields(),
        ),
        (
            "FindCutVerticesInfo",
            crate::builtin_interfaces::find_cut_vertices_info_fields(),
        ),
        (
            "BiconnectedComponentsInfo",
            crate::builtin_interfaces::biconnected_components_info_fields(),
        ),
    ] {
        builtins.insert(
            name.to_string(),
            ExportInfo {
                target: name.to_string(),
                ty: Type::Named(name.to_string()),
                interface_fields: Some(fields),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        );
    }
    builtins.insert(
        "dijkstra".to_string(),
        path_algorithm_export(
            DIJKSTRA_TARGET,
            vec![path_input(weighted_adjacency()), Type::Usize, Type::Usize],
            path_with_distance(),
            false,
        ),
    );
    builtins.insert(
        "aStar".to_string(),
        path_algorithm_export(
            A_STAR_TARGET,
            vec![
                path_input(weighted_adjacency()),
                Type::Usize,
                Type::Usize,
                i32_array(),
            ],
            path_with_distance(),
            false,
        ),
    );
    builtins.insert(
        "dijkstraInfo".to_string(),
        path_algorithm_export(
            DIJKSTRA_INFO_TARGET,
            vec![path_input(weighted_adjacency()), Type::Usize, Type::Usize],
            Type::Named("DijkstraInfo".to_string()),
            false,
        ),
    );
    builtins.insert(
        "aStarInfo".to_string(),
        path_algorithm_export(
            A_STAR_INFO_TARGET,
            vec![
                path_input(weighted_adjacency()),
                Type::Usize,
                Type::Usize,
                i32_array(),
            ],
            Type::Named("AStarInfo".to_string()),
            false,
        ),
    );
    builtins.insert(
        "bellmanFord".to_string(),
        path_algorithm_export(
            BELLMAN_FORD_TARGET,
            vec![path_input(weighted_adjacency()), Type::Usize],
            distances_with_previous(),
            true,
        ),
    );
    builtins.insert(
        "floydWarshall".to_string(),
        path_algorithm_export(
            FLOYD_WARSHALL_TARGET,
            vec![path_input(weight_matrix())],
            weight_matrix(),
            false,
        ),
    );
    builtins.insert(
        "spfa".to_string(),
        path_algorithm_export(
            SPFA_TARGET,
            vec![path_input(weighted_adjacency()), Type::Usize],
            distances_with_previous(),
            true,
        ),
    );
    builtins.insert(
        "johnson".to_string(),
        path_algorithm_export(
            JOHNSON_TARGET,
            vec![path_input(weighted_adjacency())],
            weight_matrix(),
            true,
        ),
    );
    builtins.insert(
        "bellmanFordInfo".to_string(),
        path_algorithm_export(
            BELLMAN_FORD_INFO_TARGET,
            vec![path_input(weighted_adjacency()), Type::Usize],
            Type::Named("BellmanFordInfo".to_string()),
            false,
        ),
    );
    builtins.insert(
        "spfaInfo".to_string(),
        path_algorithm_export(
            SPFA_INFO_TARGET,
            vec![path_input(weighted_adjacency()), Type::Usize],
            Type::Named("SpfaInfo".to_string()),
            true,
        ),
    );
    builtins.insert(
        "johnsonInfo".to_string(),
        path_algorithm_export(
            JOHNSON_INFO_TARGET,
            vec![path_input(weighted_adjacency())],
            Type::Named("JohnsonInfo".to_string()),
            true,
        ),
    );
    builtins.insert(
        "floydWarshallInfo".to_string(),
        path_algorithm_export(
            FLOYD_WARSHALL_INFO_TARGET,
            vec![path_input(weight_matrix())],
            Type::Named("FloydWarshallInfo".to_string()),
            false,
        ),
    );
    for (name, fields) in [
        (
            "ShortestPathInfo",
            crate::builtin_interfaces::shortest_path_info_fields(),
        ),
        (
            "DijkstraInfo",
            crate::builtin_interfaces::dijkstra_info_fields(),
        ),
        ("AStarInfo", crate::builtin_interfaces::a_star_info_fields()),
        (
            "BellmanFordInfo",
            crate::builtin_interfaces::bellman_ford_info_fields(),
        ),
        ("SpfaInfo", crate::builtin_interfaces::spfa_info_fields()),
        (
            "JohnsonInfo",
            crate::builtin_interfaces::johnson_info_fields(),
        ),
        (
            "FloydWarshallInfo",
            crate::builtin_interfaces::floyd_warshall_info_fields(),
        ),
    ] {
        builtins.insert(
            name.to_string(),
            ExportInfo {
                target: name.to_string(),
                ty: Type::Named(name.to_string()),
                interface_fields: Some(fields),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        );
    }
    builtins.insert(
        "kruskal".to_string(),
        path_algorithm_export(
            KRUSKAL_TARGET,
            vec![path_input(edge_array()), Type::Usize],
            edge_array(),
            false,
        ),
    );
    builtins.insert(
        "prim".to_string(),
        path_algorithm_export(
            PRIM_TARGET,
            vec![path_input(weighted_adjacency()), Type::Usize],
            edge_array(),
            false,
        ),
    );
    builtins.insert(
        "boruvka".to_string(),
        path_algorithm_export(
            BORUVKA_TARGET,
            vec![path_input(edge_array()), Type::Usize],
            edge_array(),
            false,
        ),
    );
    builtins.insert(
        "steinerTree".to_string(),
        path_algorithm_export(
            STEINER_TREE_TARGET,
            vec![path_input(edge_array()), Type::Usize, usize_array()],
            edge_array(),
            false,
        ),
    );
    for (name, target, params, result) in [
        (
            "kruskalInfo",
            KRUSKAL_INFO_TARGET,
            vec![path_input(edge_array()), Type::Usize],
            "KruskalInfo",
        ),
        (
            "primInfo",
            PRIM_INFO_TARGET,
            vec![path_input(weighted_adjacency()), Type::Usize],
            "PrimInfo",
        ),
        (
            "boruvkaInfo",
            BORUVKA_INFO_TARGET,
            vec![path_input(edge_array()), Type::Usize],
            "BoruvkaInfo",
        ),
        (
            "steinerTreeInfo",
            STEINER_TREE_INFO_TARGET,
            vec![path_input(edge_array()), Type::Usize, usize_array()],
            "SteinerTreeInfo",
        ),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(target, params, Type::Named(result.to_string()), false),
        );
    }
    for (name, fields) in [
        (
            "SpanningTreeInfo",
            crate::builtin_interfaces::spanning_tree_info_fields(),
        ),
        (
            "KruskalInfo",
            crate::builtin_interfaces::kruskal_info_fields(),
        ),
        ("PrimInfo", crate::builtin_interfaces::prim_info_fields()),
        (
            "BoruvkaInfo",
            crate::builtin_interfaces::boruvka_info_fields(),
        ),
        (
            "SteinerTreeInfo",
            crate::builtin_interfaces::steiner_tree_info_fields(),
        ),
    ] {
        builtins.insert(
            name.to_string(),
            ExportInfo {
                target: name.to_string(),
                ty: Type::Named(name.to_string()),
                interface_fields: Some(fields),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        );
    }
    builtins.insert(
        "edmondsKarp".to_string(),
        path_algorithm_export(
            EDMONDS_KARP_TARGET,
            vec![capacity_matrix(), Type::Usize, Type::Usize],
            flow_result(),
            false,
        ),
    );
    builtins.insert(
        "dinic".to_string(),
        path_algorithm_export(
            DINIC_TARGET,
            vec![capacity_matrix(), Type::Usize, Type::Usize],
            flow_result(),
            false,
        ),
    );
    builtins.insert(
        "pushRelabel".to_string(),
        path_algorithm_export(
            PUSH_RELABEL_TARGET,
            vec![capacity_matrix(), Type::Usize, Type::Usize],
            flow_result(),
            false,
        ),
    );
    builtins.insert(
        "minCut".to_string(),
        path_algorithm_export(
            MIN_CUT_TARGET,
            vec![capacity_matrix(), Type::Usize, Type::Usize],
            cut_result(),
            false,
        ),
    );
    builtins.insert(
        "minCostMaxFlow".to_string(),
        path_algorithm_export(
            MIN_COST_MAX_FLOW_TARGET,
            vec![
                Type::Array(Box::new(Type::Tuple(vec![
                    Type::Usize,
                    Type::Usize,
                    Type::I32,
                    Type::I32,
                ]))),
                Type::Usize,
                Type::Usize,
                Type::Usize,
            ],
            min_cost_result(),
            false,
        ),
    );
    for (name, target, params, result) in [
        (
            "edmondsKarpInfo",
            EDMONDS_KARP_INFO_TARGET,
            vec![capacity_matrix(), Type::Usize, Type::Usize],
            "EdmondsKarpInfo",
        ),
        (
            "dinicInfo",
            DINIC_INFO_TARGET,
            vec![capacity_matrix(), Type::Usize, Type::Usize],
            "DinicInfo",
        ),
        (
            "pushRelabelInfo",
            PUSH_RELABEL_INFO_TARGET,
            vec![capacity_matrix(), Type::Usize, Type::Usize],
            "PushRelabelInfo",
        ),
        (
            "minCutInfo",
            MIN_CUT_INFO_TARGET,
            vec![capacity_matrix(), Type::Usize, Type::Usize],
            "MinCutInfo",
        ),
        (
            "minCostMaxFlowInfo",
            MIN_COST_MAX_FLOW_INFO_TARGET,
            vec![
                Type::Array(Box::new(Type::Tuple(vec![
                    Type::Usize,
                    Type::Usize,
                    Type::I32,
                    Type::I32,
                ]))),
                Type::Usize,
                Type::Usize,
                Type::Usize,
            ],
            "MinCostMaxFlowInfo",
        ),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(target, params, Type::Named(result.to_string()), false),
        );
    }
    for (name, fields) in [
        ("FlowInfo", crate::builtin_interfaces::flow_info_fields()),
        (
            "EdmondsKarpInfo",
            crate::builtin_interfaces::edmonds_karp_info_fields(),
        ),
        ("DinicInfo", crate::builtin_interfaces::dinic_info_fields()),
        (
            "PushRelabelInfo",
            crate::builtin_interfaces::push_relabel_info_fields(),
        ),
        (
            "MinCutInfo",
            crate::builtin_interfaces::min_cut_info_fields(),
        ),
        (
            "MinCostMaxFlowInfo",
            crate::builtin_interfaces::min_cost_max_flow_info_fields(),
        ),
    ] {
        builtins.insert(
            name.to_string(),
            ExportInfo {
                target: name.to_string(),
                ty: Type::Named(name.to_string()),
                interface_fields: Some(fields),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        );
    }
    builtins.insert(
        "topologicalSortKahn".to_string(),
        path_algorithm_export(
            TOPOLOGICAL_KAHN_TARGET,
            vec![path_input(unweighted_adjacency())],
            optional_usize_array(),
            false,
        ),
    );
    builtins.insert(
        "topologicalSortDfs".to_string(),
        path_algorithm_export(
            TOPOLOGICAL_DFS_TARGET,
            vec![path_input(unweighted_adjacency())],
            optional_usize_array(),
            false,
        ),
    );
    for (name, target, result) in [
        (
            "topologicalSortKahnInfo",
            TOPOLOGICAL_KAHN_INFO_TARGET,
            "TopologicalSortKahnInfo",
        ),
        (
            "topologicalSortDfsInfo",
            TOPOLOGICAL_DFS_INFO_TARGET,
            "TopologicalSortDfsInfo",
        ),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(
                target,
                vec![path_input(unweighted_adjacency())],
                Type::Named(result.to_string()),
                false,
            ),
        );
    }
    for (name, fields) in [
        (
            "TopologicalSortInfo",
            crate::builtin_interfaces::topological_sort_info_fields(),
        ),
        (
            "TopologicalSortKahnInfo",
            crate::builtin_interfaces::topological_sort_kahn_info_fields(),
        ),
        (
            "TopologicalSortDfsInfo",
            crate::builtin_interfaces::topological_sort_dfs_info_fields(),
        ),
    ] {
        builtins.insert(
            name.to_string(),
            ExportInfo {
                target: name.to_string(),
                ty: Type::Named(name.to_string()),
                interface_fields: Some(fields),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        );
    }
    builtins.insert(
        "tarjanScc".to_string(),
        path_algorithm_export(
            TARJAN_SCC_TARGET,
            vec![path_input(unweighted_adjacency())],
            nested_usize_array(),
            false,
        ),
    );
    for (name, target, result) in [
        ("tarjanSccInfo", TARJAN_SCC_INFO_TARGET, "TarjanSccInfo"),
        (
            "kosarajuSccInfo",
            KOSARAJU_SCC_INFO_TARGET,
            "KosarajuSccInfo",
        ),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(
                target,
                vec![path_input(unweighted_adjacency())],
                Type::Named(result.to_string()),
                false,
            ),
        );
    }
    for (name, fields) in [
        ("SccInfo", crate::builtin_interfaces::scc_info_fields()),
        (
            "TarjanSccInfo",
            crate::builtin_interfaces::tarjan_scc_info_fields(),
        ),
        (
            "KosarajuSccInfo",
            crate::builtin_interfaces::kosaraju_scc_info_fields(),
        ),
    ] {
        builtins.insert(
            name.to_string(),
            ExportInfo {
                target: name.to_string(),
                ty: Type::Named(name.to_string()),
                interface_fields: Some(fields),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        );
    }
    builtins.insert(
        "kosarajuScc".to_string(),
        path_algorithm_export(
            KOSARAJU_SCC_TARGET,
            vec![path_input(unweighted_adjacency())],
            nested_usize_array(),
            false,
        ),
    );
    builtins.insert(
        "fleuryEulerian".to_string(),
        path_algorithm_export(
            FLEURY_EULERIAN_TARGET,
            vec![path_input(unweighted_adjacency())],
            optional_usize_array(),
            false,
        ),
    );
    builtins.insert(
        "hierholzerEulerian".to_string(),
        path_algorithm_export(
            HIERHOLZER_EULERIAN_TARGET,
            vec![path_input(unweighted_adjacency())],
            optional_usize_array(),
            false,
        ),
    );
    for (name, target, result) in [
        (
            "fleuryEulerianInfo",
            FLEURY_EULERIAN_INFO_TARGET,
            "FleuryEulerianInfo",
        ),
        (
            "hierholzerEulerianInfo",
            HIERHOLZER_EULERIAN_INFO_TARGET,
            "HierholzerEulerianInfo",
        ),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(
                target,
                vec![path_input(unweighted_adjacency())],
                Type::Named(result.to_string()),
                false,
            ),
        );
    }
    for (name, fields) in [
        (
            "EulerianInfo",
            crate::builtin_interfaces::eulerian_info_fields(),
        ),
        (
            "FleuryEulerianInfo",
            crate::builtin_interfaces::fleury_eulerian_info_fields(),
        ),
        (
            "HierholzerEulerianInfo",
            crate::builtin_interfaces::hierholzer_eulerian_info_fields(),
        ),
    ] {
        builtins.insert(
            name.to_string(),
            ExportInfo {
                target: name.to_string(),
                ty: Type::Named(name.to_string()),
                interface_fields: Some(fields),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        );
    }
    builtins.insert(
        "findBridges".to_string(),
        path_algorithm_export(
            FIND_BRIDGES_TARGET,
            vec![path_input(unweighted_adjacency())],
            pair_array(),
            false,
        ),
    );
    builtins.insert(
        "findCutVertices".to_string(),
        path_algorithm_export(
            FIND_CUT_VERTICES_TARGET,
            vec![path_input(unweighted_adjacency())],
            usize_array(),
            false,
        ),
    );
    builtins.insert(
        "biconnectedComponents".to_string(),
        path_algorithm_export(
            BICONNECTED_COMPONENTS_TARGET,
            vec![path_input(unweighted_adjacency())],
            nested_pair_array(),
            false,
        ),
    );
    builtins.insert(
        "edmondsBlossom".to_string(),
        path_algorithm_export(
            EDMONDS_BLOSSOM_TARGET,
            vec![unweighted_adjacency()],
            pair_array(),
            false,
        ),
    );
    builtins.insert(
        "hungarian".to_string(),
        path_algorithm_export(
            HUNGARIAN_TARGET,
            vec![capacity_matrix()],
            Type::Tuple(vec![Type::I32, pair_array()]),
            false,
        ),
    );
    for (name, target, params, result) in [
        (
            "edmondsBlossomInfo",
            EDMONDS_BLOSSOM_INFO_TARGET,
            vec![path_input(unweighted_adjacency())],
            "EdmondsBlossomInfo",
        ),
        (
            "hungarianInfo",
            HUNGARIAN_INFO_TARGET,
            vec![capacity_matrix()],
            "HungarianInfo",
        ),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(target, params, Type::Named(result.to_string()), false),
        );
    }
    for (name, fields) in [
        (
            "MatchingInfo",
            crate::builtin_interfaces::matching_info_fields(),
        ),
        (
            "EdmondsBlossomInfo",
            crate::builtin_interfaces::edmonds_blossom_info_fields(),
        ),
        (
            "HungarianInfo",
            crate::builtin_interfaces::hungarian_info_fields(),
        ),
    ] {
        builtins.insert(
            name.to_string(),
            ExportInfo {
                target: name.to_string(),
                ty: Type::Named(name.to_string()),
                interface_fields: Some(fields),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        );
    }
    for (name, target) in [
        ("mergeSort", MERGE_SORT_TARGET),
        ("quickSort", QUICK_SORT_TARGET),
        ("heapSort", HEAP_SORT_TARGET),
        ("radixSort", RADIX_SORT_TARGET),
        ("bucketSort", BUCKET_SORT_TARGET),
        ("timSort", TIM_SORT_TARGET),
        ("insertionSort", INSERTION_SORT_TARGET),
        ("shellSort", SHELL_SORT_TARGET),
        ("selectionSort", SELECTION_SORT_TARGET),
        ("bubbleSort", BUBBLE_SORT_TARGET),
        ("shakerSort", SHAKER_SORT_TARGET),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(target, vec![i32_array()], i32_array(), false),
        );
    }
    for (name, target) in [
        ("mergeSortInfo", MERGE_SORT_INFO_TARGET),
        ("quickSortInfo", QUICK_SORT_INFO_TARGET),
        ("heapSortInfo", HEAP_SORT_INFO_TARGET),
        ("radixSortInfo", RADIX_SORT_INFO_TARGET),
        ("bucketSortInfo", BUCKET_SORT_INFO_TARGET),
        ("timSortInfo", TIM_SORT_INFO_TARGET),
        ("insertionSortInfo", INSERTION_SORT_INFO_TARGET),
        ("shellSortInfo", SHELL_SORT_INFO_TARGET),
        ("selectionSortInfo", SELECTION_SORT_INFO_TARGET),
        ("shakerSortInfo", SHAKER_SORT_INFO_TARGET),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(
                target,
                vec![i32_array()],
                Type::Named("SortInfo".to_string()),
                false,
            ),
        );
    }
    builtins.insert(
        "countingSortInfo".to_string(),
        path_algorithm_export(
            COUNTING_SORT_INFO_TARGET,
            vec![i32_array(), Type::I32],
            Type::Named("SortInfo".to_string()),
            false,
        ),
    );
    builtins.insert(
        "kmpSearchInfo".to_string(),
        path_algorithm_export(
            KMP_SEARCH_INFO_TARGET,
            vec![Type::Str, Type::Str],
            Type::Named("KmpSearchInfo".to_string()),
            false,
        ),
    );
    builtins.insert(
        "rabinKarpInfo".to_string(),
        path_algorithm_export(
            RABIN_KARP_INFO_TARGET,
            vec![Type::Str, Type::Str],
            Type::Named("RabinKarpInfo".to_string()),
            false,
        ),
    );
    builtins.insert(
        "boyerMooreInfo".to_string(),
        path_algorithm_export(
            BOYER_MOORE_INFO_TARGET,
            vec![Type::Str, Type::Str],
            Type::Named("BoyerMooreInfo".to_string()),
            false,
        ),
    );
    builtins.insert(
        "ahoCorasickInfo".to_string(),
        path_algorithm_export(
            AHO_CORASICK_INFO_TARGET,
            vec![Type::Str, Type::Array(Box::new(Type::Str))],
            Type::Named("AhoCorasickInfo".to_string()),
            false,
        ),
    );
    builtins.insert(
        "levenshteinInfo".to_string(),
        path_algorithm_export(
            LEVENSHTEIN_INFO_TARGET,
            vec![Type::Str, Type::Str],
            Type::Named("EditDistanceInfo".to_string()),
            false,
        ),
    );
    builtins.insert(
        "RabinKarpInfo".to_string(),
        ExportInfo {
            target: "RabinKarpInfo".to_string(),
            ty: Type::Named("RabinKarpInfo".to_string()),
            interface_fields: Some(crate::builtin_interfaces::rabin_karp_info_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "BoyerMooreInfo".to_string(),
        ExportInfo {
            target: "BoyerMooreInfo".to_string(),
            ty: Type::Named("BoyerMooreInfo".to_string()),
            interface_fields: Some(crate::builtin_interfaces::boyer_moore_info_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "AhoCorasickInfo".to_string(),
        ExportInfo {
            target: "AhoCorasickInfo".to_string(),
            ty: Type::Named("AhoCorasickInfo".to_string()),
            interface_fields: Some(crate::builtin_interfaces::aho_corasick_info_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "EditDistanceInfo".to_string(),
        ExportInfo {
            target: "EditDistanceInfo".to_string(),
            ty: Type::Named("EditDistanceInfo".to_string()),
            interface_fields: Some(crate::builtin_interfaces::edit_distance_info_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    for (name, fields) in [
        (
            "StringSearchInfo",
            crate::builtin_interfaces::string_search_info_fields(),
        ),
        (
            "KmpSearchInfo",
            crate::builtin_interfaces::kmp_search_info_fields(),
        ),
    ] {
        builtins.insert(
            name.to_string(),
            ExportInfo {
                target: name.to_string(),
                ty: Type::Named(name.to_string()),
                interface_fields: Some(fields),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        );
    }
    builtins.insert(
        "bubbleSortInfo".to_string(),
        path_algorithm_export(
            BUBBLE_SORT_INFO_TARGET,
            vec![i32_array()],
            Type::Named("BubbleSortInfo".to_string()),
            false,
        ),
    );
    builtins.insert(
        "SortInfo".to_string(),
        ExportInfo {
            target: "SortInfo".to_string(),
            ty: Type::Named("SortInfo".to_string()),
            interface_fields: Some(crate::builtin_interfaces::sort_info_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "BubbleSortInfo".to_string(),
        ExportInfo {
            target: "BubbleSortInfo".to_string(),
            ty: Type::Named("BubbleSortInfo".to_string()),
            interface_fields: Some(crate::builtin_interfaces::bubble_sort_info_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "countingSort".to_string(),
        path_algorithm_export(
            COUNTING_SORT_TARGET,
            vec![i32_array(), Type::I32],
            i32_array(),
            false,
        ),
    );
    for (name, target) in [
        ("binarySearch", BINARY_SEARCH_TARGET),
        ("interpolationSearch", INTERPOLATION_SEARCH_TARGET),
        ("exponentialSearch", EXPONENTIAL_SEARCH_TARGET),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(
                target,
                vec![i32_array(), Type::I32],
                Type::Optional(Box::new(Type::Usize)),
                false,
            ),
        );
    }
    builtins.insert(
        "quickselect".to_string(),
        path_algorithm_export(
            QUICKSELECT_TARGET,
            vec![i32_array(), Type::Usize],
            Type::Optional(Box::new(Type::I32)),
            false,
        ),
    );
    for (name, target, result) in [
        ("binarySearchInfo", BINARY_SEARCH_INFO_TARGET, "SearchInfo"),
        (
            "interpolationSearchInfo",
            INTERPOLATION_SEARCH_INFO_TARGET,
            "InterpolationSearchInfo",
        ),
        (
            "exponentialSearchInfo",
            EXPONENTIAL_SEARCH_INFO_TARGET,
            "ExponentialSearchInfo",
        ),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(
                target,
                vec![i32_array(), Type::I32],
                Type::Named(result.to_string()),
                false,
            ),
        );
    }
    builtins.insert(
        "quickselectInfo".to_string(),
        path_algorithm_export(
            QUICKSELECT_INFO_TARGET,
            vec![i32_array(), Type::Usize],
            Type::Named("SelectionInfo".to_string()),
            false,
        ),
    );
    for (name, fields) in [
        (
            "SearchInfo",
            crate::builtin_interfaces::search_info_fields(),
        ),
        (
            "InterpolationSearchInfo",
            crate::builtin_interfaces::interpolation_search_info_fields(),
        ),
        (
            "ExponentialSearchInfo",
            crate::builtin_interfaces::exponential_search_info_fields(),
        ),
        (
            "SelectionInfo",
            crate::builtin_interfaces::selection_info_fields(),
        ),
    ] {
        builtins.insert(
            name.to_string(),
            ExportInfo {
                target: name.to_string(),
                ty: Type::Named(name.to_string()),
                interface_fields: Some(fields),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        );
    }
    for (name, target) in [
        ("kmpSearch", KMP_SEARCH_TARGET),
        ("rabinKarp", RABIN_KARP_TARGET),
        ("boyerMoore", BOYER_MOORE_TARGET),
    ] {
        builtins.insert(
            name.to_string(),
            path_algorithm_export(target, vec![Type::Str, Type::Str], usize_array(), false),
        );
    }
    builtins.insert(
        "zAlgorithm".to_string(),
        path_algorithm_export(Z_ALGORITHM_TARGET, vec![Type::Str], usize_array(), false),
    );
    builtins.insert(
        "zAlgorithmInfo".to_string(),
        path_algorithm_export(
            Z_ALGORITHM_INFO_TARGET,
            vec![Type::Str],
            Type::Named("ZAlgorithmInfo".to_string()),
            false,
        ),
    );
    builtins.insert(
        "ZAlgorithmInfo".to_string(),
        ExportInfo {
            target: "ZAlgorithmInfo".to_string(),
            ty: Type::Named("ZAlgorithmInfo".to_string()),
            interface_fields: Some(crate::builtin_interfaces::z_algorithm_info_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "ahoCorasick".to_string(),
        path_algorithm_export(
            AHO_CORASICK_TARGET,
            vec![Type::Str, Type::Array(Box::new(Type::Str))],
            pair_array(),
            false,
        ),
    );
    builtins.insert(
        "levenshtein".to_string(),
        path_algorithm_export(
            LEVENSHTEIN_TARGET,
            vec![Type::Str, Type::Str],
            Type::Usize,
            false,
        ),
    );
    builtins.insert(
        "AlgorithmError".to_string(),
        ExportInfo {
            target: "AlgorithmError".to_string(),
            ty: algorithm_error(),
            interface_fields: Some(algorithm_error_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "UnionFind".to_string(),
        ExportInfo {
            target: UNION_FIND_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![Type::Usize],
                ret: Box::new(Type::Named("UnionFind".to_string())),
            },
            interface_fields: Some(union_find_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "BinaryHeap".to_string(),
        ExportInfo {
            target: BINARY_HEAP_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("BinaryHeap".to_string())),
            },
            interface_fields: Some(binary_heap_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "BinomialHeap".to_string(),
        ExportInfo {
            target: BINOMIAL_HEAP_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("BinomialHeap".to_string())),
            },
            interface_fields: Some(binomial_heap_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "Counter".to_string(),
        ExportInfo {
            target: COUNTER_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("Counter".to_string())),
            },
            interface_fields: Some(counter_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "Deque".to_string(),
        ExportInfo {
            target: DEQUE_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("Deque".to_string())),
            },
            interface_fields: Some(deque_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "FenwickTree".to_string(),
        ExportInfo {
            target: FENWICK_TREE_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![Type::Usize],
                ret: Box::new(Type::Named("FenwickTree".to_string())),
            },
            interface_fields: Some(fenwick_tree_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "SegmentTree".to_string(),
        ExportInfo {
            target: SEGMENT_TREE_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![Type::Array(Box::new(Type::I32))],
                ret: Box::new(Type::Named("SegmentTree".to_string())),
            },
            interface_fields: Some(segment_tree_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "SparseTable".to_string(),
        ExportInfo {
            target: SPARSE_TABLE_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![Type::Array(Box::new(Type::I32))],
                ret: Box::new(Type::Named("SparseTable".to_string())),
            },
            interface_fields: Some(sparse_table_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "SegmentTreeLazy".to_string(),
        ExportInfo {
            target: SEGMENT_TREE_LAZY_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![Type::Array(Box::new(Type::I32))],
                ret: Box::new(Type::Named("SegmentTreeLazy".to_string())),
            },
            interface_fields: Some(segment_tree_lazy_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "BitSet".to_string(),
        ExportInfo {
            target: BIT_SET_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![Type::Usize],
                ret: Box::new(Type::Named("BitSet".to_string())),
            },
            interface_fields: Some(bit_set_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "LinkedList".to_string(),
        ExportInfo {
            target: LINKED_LIST_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("LinkedList".to_string())),
            },
            interface_fields: Some(linked_list_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    builtins.insert(
        "DoubleLinkedList".to_string(),
        ExportInfo {
            target: DOUBLE_LINKED_LIST_NEW_TARGET.to_string(),
            ty: Type::Function {
                params: vec![],
                ret: Box::new(Type::Named("DoubleLinkedList".to_string())),
            },
            interface_fields: Some(double_linked_list_interface_fields()),
            behavior_info: None,
            type_alias_ty: None,
            enum_info: None,
            throws: vec![],
            db_model: None,
        },
    );
    for (name, target, fields) in [
        ("AvlTree", AVL_TREE_NEW_TARGET, avl_tree_interface_fields()),
        (
            "RedBlackTree",
            RED_BLACK_TREE_NEW_TARGET,
            red_black_tree_interface_fields(),
        ),
        (
            "SplayTree",
            SPLAY_TREE_NEW_TARGET,
            splay_tree_interface_fields(),
        ),
        ("Treap", TREAP_NEW_TARGET, treap_interface_fields()),
        (
            "SortedSet",
            SORTED_SET_NEW_TARGET,
            sorted_set_interface_fields(),
        ),
        (
            "SkipList",
            SKIP_LIST_NEW_TARGET,
            skip_list_interface_fields(),
        ),
        (
            "SortedMap",
            SORTED_MAP_NEW_TARGET,
            sorted_map_interface_fields(),
        ),
        ("Graph", GRAPH_NEW_TARGET, graph_interface_fields()),
        ("Trie", TRIE_NEW_TARGET, trie_interface_fields()),
    ] {
        builtins.insert(
            name.to_string(),
            ExportInfo {
                target: target.to_string(),
                ty: Type::Function {
                    params: vec![],
                    ret: Box::new(Type::Named(name.to_string())),
                },
                interface_fields: Some(fields),
                behavior_info: None,
                type_alias_ty: None,
                enum_info: None,
                throws: vec![],
                db_model: None,
            },
        );
    }
    builtins
}
