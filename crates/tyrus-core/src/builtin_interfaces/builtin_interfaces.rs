//! Built-in interfaces shared by the type checker and codegen.
//!
//! Both consumers need the exact same field list, in the exact same order,
//! for every interface registered here: the type checker uses it for
//! member-access type checking, and codegen's `llvm_type` lowers a
//! `Type::Named` interface to an LLVM struct whose field order must match
//! what the runtime's `#[repr(C)]` FFI structs (or, for `HttpRequest`, the
//! native HTTP server) actually produce. A single source keeps the two from
//! drifting out of sync, see `crates/tyrus-core/src/type_checker/type_checker.rs` and
//! `crates/tyrus-codegen/src/core/core.rs`.
//!
//! An interface whose fields are themselves parameterized by another native
//! module (e.g. crypto's namespace interfaces, CLI's prompt option
//! interfaces) is not here, each of those already has its own dedicated,
//! already-shared function (`crypto::crypto_namespace_interface_fields`,
//! `cli::option_interfaces`) that both consumers call directly.
//!
//! Codegen calls [`builtin_interfaces`] and always gets every interface
//! here: it needs a native call's full struct layout to generate correct IR
//! regardless of which type names the file being compiled happens to
//! import. The type checker calls [`ambient_builtin_interfaces`] instead,
//! a deliberately smaller set. An interface tied to a language feature
//! (`@validate`'s `ValidationIssue`/`ValidationError`) or an ambient
//! namespace (`process`'s `MemoryInfo`/`CpuUsage`) has nothing to import, so
//! it stays ambient; `tyrus:os`'s own interfaces (`UserInfo`, `CpuInfo`,
//! `BatteryInfo`, `NetworkInterface`) do not, the checker only accepts
//! those by name once the file imports them, registered as importable
//! exports in `os_builtins` (`os.rs`), which calls the same per-interface
//! field functions below to avoid a second, divergent field list.

use crate::ast::{Expr, ExprKind, Field, ResolvedEnum, Type};
use crate::collections::HashMap;
use crate::span::Span;

fn field(name: &str, ty: Type) -> Field {
    Field {
        name: name.to_string(),
        ty,
        readonly: false,
        optional: false,
        decorators: Vec::new(),
        span: Span::DUMMY,
    }
}

fn readonly_field(name: &str, ty: Type) -> Field {
    Field {
        readonly: true,
        ..field(name, ty)
    }
}

fn optional_field(name: &str, ty: Type) -> Field {
    Field {
        optional: true,
        ..field(name, ty)
    }
}

fn string_literal_union(values: &[&str]) -> Type {
    Type::Union(
        values
            .iter()
            .map(|value| {
                Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(
                    (*value).to_string(),
                ))))
            })
            .collect(),
    )
}

pub fn cors_method_type() -> Type {
    string_literal_union(&[
        "CONNECT", "DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT", "QUERY", "TRACE",
    ])
}

pub fn cookie_same_site_type() -> Type {
    string_literal_union(&["lax", "strict", "none"])
}

pub fn user_info_fields() -> Vec<Field> {
    vec![
        readonly_field("username", Type::Str),
        readonly_field("homedir", Type::Str),
        readonly_field("shell", Type::Str),
    ]
}

pub fn cpu_info_fields() -> Vec<Field> {
    vec![
        readonly_field("model", Type::Str),
        readonly_field("speed", Type::U32),
    ]
}

pub fn battery_info_fields() -> Vec<Field> {
    vec![
        readonly_field("level", Type::F32),
        readonly_field("charging", Type::Bool),
        readonly_field("timeToFull", Type::Optional(Box::new(Type::U32))),
        readonly_field("timeToEmpty", Type::Optional(Box::new(Type::U32))),
    ]
}

pub fn network_interface_fields() -> Vec<Field> {
    vec![
        readonly_field("name", Type::Str),
        readonly_field("address", Type::Str),
        readonly_field("netmask", Type::Str),
        readonly_field("family", Type::Str),
    ]
}

pub fn simd_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn os_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn archive_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
        readonly_field("entry", Type::Optional(Box::new(Type::Str))),
    ]
}

pub fn archive_entry_fields() -> Vec<Field> {
    vec![
        readonly_field("name", Type::Str),
        readonly_field("kind", Type::Named("ArchiveEntryKind".to_string())),
        readonly_field("size", Type::U64),
        readonly_field("compressedSize", Type::Optional(Box::new(Type::U64))),
    ]
}

pub fn archive_summary_fields() -> Vec<Field> {
    vec![
        readonly_field("entries", Type::U64),
        readonly_field("bytes", Type::U64),
    ]
}

pub fn pack_options_fields() -> Vec<Field> {
    vec![
        optional_field("level", Type::I32),
        optional_field("overwrite", Type::Bool),
        optional_field("exclude", Type::Array(Box::new(Type::Str))),
        optional_field("prefix", Type::Str),
        optional_field("reproducible", Type::Bool),
        optional_field("timeoutMs", Type::U64),
    ]
}

pub fn unpack_options_fields() -> Vec<Field> {
    vec![
        optional_field("overwrite", Type::Bool),
        optional_field("stripComponents", Type::U32),
        optional_field("maxEntries", Type::U64),
        optional_field("maxBytes", Type::U64),
        optional_field("maxRatio", Type::U32),
        optional_field("timeoutMs", Type::U64),
    ]
}

pub fn compress_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn db_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn compress_options_fields() -> Vec<Field> {
    vec![optional_field("level", Type::I32)]
}

pub fn decompress_options_fields() -> Vec<Field> {
    vec![
        optional_field("format", Type::Named("CompressFormat".to_string())),
        optional_field("maxOutputBytes", Type::U64),
        optional_field("maxRatio", Type::U32),
    ]
}

pub fn db_page_fields() -> Vec<Field> {
    vec![
        readonly_field(
            "items",
            Type::Array(Box::new(Type::Generic("T".to_string()))),
        ),
        readonly_field("total", Type::U64),
        readonly_field("pageSize", Type::U64),
        optional_field("page", Type::U64),
        optional_field("totalPages", Type::U64),
        optional_field("nextCursor", Type::Str),
        optional_field("hasMore", Type::Bool),
    ]
}

pub fn zip_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn http_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn native_error_fields(name: &str) -> Option<Vec<Field>> {
    match name {
        "ArchiveError" => Some(archive_error_fields()),
        "CompressError" => Some(compress_error_fields()),
        "DbError" => Some(db_error_fields()),
        "EncodingError" => Some(encoding_error_fields()),
        "FsError" => Some(fs_error_fields()),
        "HttpError" => Some(http_error_fields()),
        "MemoryError" => Some(memory_error_fields()),
        "NetError" => Some(net_error_fields()),
        "OsError" => Some(os_error_fields()),
        "ProcessError" => Some(process_error_fields()),
        "StreamError" => Some(stream_error_fields()),
        "TemporalError" => Some(temporal_error_fields()),
        _ => None,
    }
}

pub fn stream_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn process_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn child_process_fields() -> Vec<Field> {
    let bytes = Type::Array(Box::new(Type::U8));
    vec![
        readonly_field("pid", Type::U32),
        readonly_field(
            "stdin",
            Type::Apply {
                name: "WritableStream".to_string(),
                args: vec![bytes.clone()],
            },
        ),
        readonly_field(
            "stdout",
            Type::Apply {
                name: "ReadableStream".to_string(),
                args: vec![bytes.clone()],
            },
        ),
        readonly_field(
            "stderr",
            Type::Apply {
                name: "ReadableStream".to_string(),
                args: vec![bytes],
            },
        ),
    ]
}

pub fn encoding_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn crypto_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn memory_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn sort_info_fields() -> Vec<Field> {
    vec![
        readonly_field("originalValues", Type::Array(Box::new(Type::I32))),
        readonly_field("sortedValues", Type::Array(Box::new(Type::I32))),
        readonly_field("comparisonCount", Type::Usize),
        readonly_field("swapCount", Type::Usize),
    ]
}

pub fn bubble_sort_info_fields() -> Vec<Field> {
    let mut fields = sort_info_fields();
    fields.push(readonly_field("passCount", Type::Usize));
    fields
}

pub fn search_info_fields() -> Vec<Field> {
    vec![
        readonly_field("values", Type::Array(Box::new(Type::I32))),
        readonly_field("target", Type::I32),
        readonly_field("index", Type::Optional(Box::new(Type::Usize))),
        readonly_field("comparisonCount", Type::Usize),
        readonly_field("probeCount", Type::Usize),
    ]
}

pub fn interpolation_search_info_fields() -> Vec<Field> {
    let mut fields = search_info_fields();
    fields.push(readonly_field(
        "estimatedPositions",
        Type::Array(Box::new(Type::Usize)),
    ));
    fields
}

pub fn exponential_search_info_fields() -> Vec<Field> {
    let mut fields = search_info_fields();
    fields.push(readonly_field("rangeExpansions", Type::Usize));
    fields.push(readonly_field("finalLow", Type::Usize));
    fields.push(readonly_field("finalHigh", Type::Usize));
    fields
}

pub fn selection_info_fields() -> Vec<Field> {
    vec![
        readonly_field("values", Type::Array(Box::new(Type::I32))),
        readonly_field("selected", Type::Optional(Box::new(Type::I32))),
        readonly_field("rank", Type::Usize),
        readonly_field("comparisonCount", Type::Usize),
        readonly_field("swapCount", Type::Usize),
        readonly_field("partitionCount", Type::Usize),
    ]
}

pub fn traversal_info_fields() -> Vec<Field> {
    vec![
        readonly_field("start", Type::Usize),
        readonly_field("visitOrder", Type::Array(Box::new(Type::Usize))),
        readonly_field("visitedCount", Type::Usize),
        readonly_field("examinedEdgeCount", Type::Usize),
        readonly_field("maxFrontierSize", Type::Usize),
        readonly_field("maxDepth", Type::Usize),
    ]
}

pub fn iddfs_info_fields() -> Vec<Field> {
    let mut fields = traversal_info_fields();
    fields.push(readonly_field(
        "depthLimitsTried",
        Type::Array(Box::new(Type::Usize)),
    ));
    fields.push(readonly_field("iterationCount", Type::Usize));
    fields
}

pub fn shortest_path_info_fields() -> Vec<Field> {
    vec![
        readonly_field("distances", Type::Array(Box::new(Type::I32))),
        readonly_field(
            "predecessors",
            Type::Array(Box::new(Type::Optional(Box::new(Type::Usize)))),
        ),
        readonly_field("relaxationCount", Type::Usize),
        readonly_field("processedVertexCount", Type::Usize),
        readonly_field("examinedEdgeCount", Type::Usize),
    ]
}

pub fn dijkstra_info_fields() -> Vec<Field> {
    let mut fields = shortest_path_info_fields();
    fields.push(readonly_field("heapExtractionCount", Type::Usize));
    fields.push(readonly_field("acceptedRelaxationCount", Type::Usize));
    fields.push(readonly_field("finalizedVertexCount", Type::Usize));
    fields
}

pub fn a_star_info_fields() -> Vec<Field> {
    let mut fields = shortest_path_info_fields();
    fields.push(readonly_field("heuristicEvaluationCount", Type::Usize));
    fields.push(readonly_field("openedVertexCount", Type::Usize));
    fields.push(readonly_field("closedVertexCount", Type::Usize));
    fields.push(readonly_field(
        "finalCost",
        Type::Optional(Box::new(Type::I32)),
    ));
    fields
}

pub fn bellman_ford_info_fields() -> Vec<Field> {
    let mut fields = shortest_path_info_fields();
    fields.push(readonly_field("passCount", Type::Usize));
    fields.push(readonly_field("negativeCycleDetected", Type::Bool));
    fields.push(readonly_field(
        "negativeCycleEdge",
        Type::Optional(Box::new(Type::Tuple(vec![
            Type::Usize,
            Type::Usize,
            Type::I32,
        ]))),
    ));
    fields
}

pub fn spfa_info_fields() -> Vec<Field> {
    let mut fields = shortest_path_info_fields();
    fields.push(readonly_field("queuePushCount", Type::Usize));
    fields.push(readonly_field("queuePopCount", Type::Usize));
    fields.push(readonly_field("maxQueueSize", Type::Usize));
    fields
}

pub fn johnson_info_fields() -> Vec<Field> {
    vec![
        readonly_field(
            "distances",
            Type::Array(Box::new(Type::Array(Box::new(Type::Optional(Box::new(
                Type::I32,
            )))))),
        ),
        readonly_field("reweightingCount", Type::Usize),
        readonly_field("bellmanFordRelaxationCount", Type::Usize),
        readonly_field("dijkstraExecutionCount", Type::Usize),
    ]
}

pub fn floyd_warshall_info_fields() -> Vec<Field> {
    vec![
        readonly_field(
            "distances",
            Type::Array(Box::new(Type::Array(Box::new(Type::Optional(Box::new(
                Type::I32,
            )))))),
        ),
        readonly_field("kPassCount", Type::Usize),
        readonly_field("distanceUpdateCount", Type::Usize),
        readonly_field("negativeCycleDetected", Type::Bool),
    ]
}

pub fn spanning_tree_info_fields() -> Vec<Field> {
    vec![
        readonly_field(
            "edges",
            Type::Array(Box::new(Type::Tuple(vec![
                Type::Usize,
                Type::Usize,
                Type::I32,
            ]))),
        ),
        readonly_field("totalWeight", Type::I32),
        readonly_field("componentCount", Type::Usize),
        readonly_field("examinedEdgeCount", Type::Usize),
        readonly_field("acceptedEdgeCount", Type::Usize),
    ]
}

pub fn flow_info_fields() -> Vec<Field> {
    vec![
        readonly_field("maxFlow", Type::I32),
        readonly_field("augmentationCount", Type::Usize),
        readonly_field("examinedEdgeCount", Type::Usize),
        readonly_field(
            "flowEdges",
            Type::Array(Box::new(Type::Tuple(vec![
                Type::Usize,
                Type::Usize,
                Type::I32,
            ]))),
        ),
    ]
}

pub fn topological_sort_info_fields() -> Vec<Field> {
    vec![
        readonly_field("order", Type::Array(Box::new(Type::Usize))),
        readonly_field("processedVertexCount", Type::Usize),
        readonly_field("examinedEdgeCount", Type::Usize),
        readonly_field("cycleDetected", Type::Bool),
    ]
}

pub fn topological_sort_kahn_info_fields() -> Vec<Field> {
    let mut fields = topological_sort_info_fields();
    fields.extend([
        readonly_field("initialIndegrees", Type::Array(Box::new(Type::Usize))),
        readonly_field("indegreeDecrementCount", Type::Usize),
        readonly_field("maxQueueSize", Type::Usize),
    ]);
    fields
}

pub fn topological_sort_dfs_info_fields() -> Vec<Field> {
    let mut fields = topological_sort_info_fields();
    fields.extend([
        readonly_field("finalStates", Type::Array(Box::new(Type::Usize))),
        readonly_field("maxDepth", Type::Usize),
        readonly_field(
            "backEdge",
            Type::Optional(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize]))),
        ),
    ]);
    fields
}

pub fn scc_info_fields() -> Vec<Field> {
    vec![
        readonly_field(
            "components",
            Type::Array(Box::new(Type::Array(Box::new(Type::Usize)))),
        ),
        readonly_field("componentCount", Type::Usize),
        readonly_field("examinedEdgeCount", Type::Usize),
        readonly_field("stackOperationCount", Type::Usize),
    ]
}
pub fn tarjan_scc_info_fields() -> Vec<Field> {
    let mut fields = scc_info_fields();
    fields.extend([
        readonly_field("discoveryIndices", Type::Array(Box::new(Type::Usize))),
        readonly_field("lowLinks", Type::Array(Box::new(Type::Usize))),
        readonly_field("stackPushCount", Type::Usize),
        readonly_field("stackPopCount", Type::Usize),
    ]);
    fields
}
pub fn kosaraju_scc_info_fields() -> Vec<Field> {
    let mut fields = scc_info_fields();
    fields.extend([
        readonly_field("finishingOrder", Type::Array(Box::new(Type::Usize))),
        readonly_field("firstTraversalEdgeCount", Type::Usize),
        readonly_field("secondTraversalEdgeCount", Type::Usize),
    ]);
    fields
}

pub fn eulerian_info_fields() -> Vec<Field> {
    vec![
        readonly_field(
            "path",
            Type::Optional(Box::new(Type::Array(Box::new(Type::Usize)))),
        ),
        readonly_field("usedEdgeCount", Type::Usize),
        readonly_field("examinedEdgeCount", Type::Usize),
        readonly_field("isCircuit", Type::Bool),
    ]
}
pub fn fleury_eulerian_info_fields() -> Vec<Field> {
    let mut fields = eulerian_info_fields();
    fields.extend([
        readonly_field("bridgeCheckCount", Type::Usize),
        readonly_field("avoidedBridgeChoiceCount", Type::Usize),
    ]);
    fields
}
pub fn hierholzer_eulerian_info_fields() -> Vec<Field> {
    let mut fields = eulerian_info_fields();
    fields.extend([
        readonly_field("stackPushCount", Type::Usize),
        readonly_field("stackPopCount", Type::Usize),
        readonly_field("joinedCycleCount", Type::Usize),
    ]);
    fields
}

pub fn connectivity_info_fields() -> Vec<Field> {
    vec![
        readonly_field("examinedEdgeCount", Type::Usize),
        readonly_field("discoveryIndices", Type::Array(Box::new(Type::Usize))),
        readonly_field("lowLinks", Type::Array(Box::new(Type::Usize))),
        readonly_field("resultCount", Type::Usize),
    ]
}
pub fn find_bridges_info_fields() -> Vec<Field> {
    let mut fields = connectivity_info_fields();
    fields.push(readonly_field(
        "bridges",
        Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize]))),
    ));
    fields
}
pub fn find_cut_vertices_info_fields() -> Vec<Field> {
    let mut fields = connectivity_info_fields();
    fields.push(readonly_field(
        "cutVertices",
        Type::Array(Box::new(Type::Usize)),
    ));
    fields
}
pub fn biconnected_components_info_fields() -> Vec<Field> {
    let mut fields = connectivity_info_fields();
    fields.push(readonly_field(
        "components",
        Type::Array(Box::new(Type::Array(Box::new(Type::Tuple(vec![
            Type::Usize,
            Type::Usize,
        ]))))),
    ));
    fields
}
pub fn matching_info_fields() -> Vec<Field> {
    vec![
        readonly_field(
            "pairs",
            Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize]))),
        ),
        readonly_field("cardinality", Type::Usize),
        readonly_field("examinedEdgeCount", Type::Usize),
    ]
}
pub fn string_search_info_fields() -> Vec<Field> {
    vec![
        readonly_field("text", Type::Str),
        readonly_field("pattern", Type::Str),
        readonly_field("matches", Type::Array(Box::new(Type::Usize))),
        readonly_field("comparisonCount", Type::Usize),
        readonly_field("shiftCount", Type::Usize),
    ]
}
pub fn kmp_search_info_fields() -> Vec<Field> {
    let mut fields = string_search_info_fields();
    fields.extend([
        readonly_field("prefixTable", Type::Array(Box::new(Type::Usize))),
        readonly_field("prefixFallbackCount", Type::Usize),
    ]);
    fields
}
pub fn z_algorithm_info_fields() -> Vec<Field> {
    vec![
        readonly_field("text", Type::Str),
        readonly_field("zTable", Type::Array(Box::new(Type::Usize))),
        readonly_field("comparisonCount", Type::Usize),
        readonly_field("windowExtensionCount", Type::Usize),
    ]
}
pub fn rabin_karp_info_fields() -> Vec<Field> {
    let mut fields = string_search_info_fields();
    fields.extend([
        readonly_field("windowCount", Type::Usize),
        readonly_field("hashCollisionCount", Type::Usize),
        readonly_field("textVerificationCount", Type::Usize),
    ]);
    fields
}
pub fn boyer_moore_info_fields() -> Vec<Field> {
    let mut fields = string_search_info_fields();
    fields.extend([
        readonly_field(
            "badCharacterTable",
            Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize]))),
        ),
        readonly_field("appliedShifts", Type::Array(Box::new(Type::Usize))),
    ]);
    fields
}
pub fn aho_corasick_info_fields() -> Vec<Field> {
    vec![
        readonly_field("text", Type::Str),
        readonly_field("patterns", Type::Array(Box::new(Type::Str))),
        readonly_field(
            "matches",
            Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize]))),
        ),
        readonly_field("visitedStates", Type::Array(Box::new(Type::Usize))),
        readonly_field("transitionCount", Type::Usize),
        readonly_field("failureFollowCount", Type::Usize),
    ]
}
pub fn edit_distance_info_fields() -> Vec<Field> {
    vec![
        readonly_field("left", Type::Str),
        readonly_field("right", Type::Str),
        readonly_field("distance", Type::Usize),
        readonly_field("rowCount", Type::Usize),
        readonly_field("columnCount", Type::Usize),
        readonly_field("evaluatedCellCount", Type::Usize),
    ]
}
pub fn suffix_array_info_fields() -> Vec<Field> {
    vec![
        readonly_field("text", Type::Str),
        readonly_field("suffixArray", Type::Array(Box::new(Type::Usize))),
        readonly_field("sortRoundCount", Type::Usize),
        readonly_field("comparisonCount", Type::Usize),
        readonly_field("comparedByteCount", Type::Usize),
    ]
}
pub fn edmonds_blossom_info_fields() -> Vec<Field> {
    let mut fields = matching_info_fields();
    fields.extend([
        readonly_field("blossomContractionCount", Type::Usize),
        readonly_field("augmentingPathSearchCount", Type::Usize),
        readonly_field("augmentationCount", Type::Usize),
    ]);
    fields
}
pub fn hungarian_info_fields() -> Vec<Field> {
    vec![
        readonly_field(
            "assignments",
            Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize]))),
        ),
        readonly_field("totalCost", Type::I32),
        readonly_field("potentialAdjustmentCount", Type::Usize),
        readonly_field("matrixIterationCount", Type::Usize),
    ]
}

pub fn edmonds_karp_info_fields() -> Vec<Field> {
    let mut fields = flow_info_fields();
    fields.extend([
        readonly_field("bfsRunCount", Type::Usize),
        readonly_field("augmentingPathLengths", Type::Array(Box::new(Type::Usize))),
        readonly_field("maxFrontierSize", Type::Usize),
    ]);
    fields
}

pub fn dinic_info_fields() -> Vec<Field> {
    let mut fields = flow_info_fields();
    fields.extend([
        readonly_field("levelGraphCount", Type::Usize),
        readonly_field("blockingFlowCount", Type::Usize),
        readonly_field("currentArcAdvanceCount", Type::Usize),
    ]);
    fields
}

pub fn push_relabel_info_fields() -> Vec<Field> {
    let mut fields = flow_info_fields();
    fields.extend([
        readonly_field("pushCount", Type::Usize),
        readonly_field("relabelCount", Type::Usize),
        readonly_field("dischargeCount", Type::Usize),
        readonly_field("maxHeight", Type::Usize),
    ]);
    fields
}

pub fn min_cut_info_fields() -> Vec<Field> {
    vec![
        readonly_field("cutValue", Type::I32),
        readonly_field("sourcePartition", Type::Array(Box::new(Type::Usize))),
        readonly_field("sinkPartition", Type::Array(Box::new(Type::Usize))),
        readonly_field(
            "cutEdges",
            Type::Array(Box::new(Type::Tuple(vec![Type::Usize, Type::Usize]))),
        ),
        readonly_field(
            "flowEdges",
            Type::Array(Box::new(Type::Tuple(vec![
                Type::Usize,
                Type::Usize,
                Type::I32,
            ]))),
        ),
    ]
}

pub fn min_cost_max_flow_info_fields() -> Vec<Field> {
    vec![
        readonly_field("maxFlow", Type::I32),
        readonly_field("minimumCost", Type::I32),
        readonly_field("augmentationCount", Type::Usize),
        readonly_field("costRelaxationCount", Type::Usize),
        readonly_field("cumulativeCosts", Type::Array(Box::new(Type::I32))),
    ]
}

pub fn kruskal_info_fields() -> Vec<Field> {
    let mut fields = spanning_tree_info_fields();
    fields.push(readonly_field("unionAttemptCount", Type::Usize));
    fields.push(readonly_field("successfulUnionCount", Type::Usize));
    fields.push(readonly_field("sortedEdgeExaminationCount", Type::Usize));
    fields
}

pub fn prim_info_fields() -> Vec<Field> {
    let mut fields = spanning_tree_info_fields();
    fields.push(readonly_field("heapPushCount", Type::Usize));
    fields.push(readonly_field("heapPopCount", Type::Usize));
    fields.push(readonly_field("keyUpdateCount", Type::Usize));
    fields
}

pub fn boruvka_info_fields() -> Vec<Field> {
    let mut fields = spanning_tree_info_fields();
    fields.push(readonly_field("phaseCount", Type::Usize));
    fields.push(readonly_field(
        "componentCountsAfterPhase",
        Type::Array(Box::new(Type::Usize)),
    ));
    fields
}

pub fn steiner_tree_info_fields() -> Vec<Field> {
    vec![
        readonly_field(
            "edges",
            Type::Array(Box::new(Type::Tuple(vec![
                Type::Usize,
                Type::Usize,
                Type::I32,
            ]))),
        ),
        readonly_field("totalWeight", Type::I32),
        readonly_field("terminals", Type::Array(Box::new(Type::Usize))),
        readonly_field("evaluatedSubsetCount", Type::Usize),
        readonly_field("vertexCount", Type::Usize),
        readonly_field("maxTerminalCount", Type::Usize),
        readonly_field("withinTerminalLimit", Type::Bool),
    ]
}

pub fn native_error_extends_error(name: &str) -> bool {
    matches!(
        name,
        "AlgorithmError"
            | "ArchiveError"
            | "CompressError"
            | "EncodingError"
            | "FsError"
            | "HttpError"
            | "JsonError"
            | "MemoryError"
            | "NetError"
            | "OsError"
            | "ProcessError"
            | "StreamError"
            | "TemporalError"
    )
}

pub fn interface_extends(interface: &str, base: &str) -> bool {
    matches!(
        (interface, base),
        ("BubbleSortInfo", "SortInfo")
            | (
                "InterpolationSearchInfo" | "ExponentialSearchInfo",
                "SearchInfo"
            )
            | ("IddfsInfo", "TraversalInfo")
            | (
                "DijkstraInfo" | "AStarInfo" | "BellmanFordInfo" | "SpfaInfo",
                "ShortestPathInfo"
            )
            | (
                "KruskalInfo" | "PrimInfo" | "BoruvkaInfo",
                "SpanningTreeInfo"
            )
            | (
                "EdmondsKarpInfo" | "DinicInfo" | "PushRelabelInfo",
                "FlowInfo"
            )
            | (
                "TopologicalSortKahnInfo" | "TopologicalSortDfsInfo",
                "TopologicalSortInfo"
            )
            | ("TarjanSccInfo" | "KosarajuSccInfo", "SccInfo")
            | (
                "FleuryEulerianInfo" | "HierholzerEulerianInfo",
                "EulerianInfo"
            )
            | (
                "FindBridgesInfo" | "FindCutVerticesInfo" | "BiconnectedComponentsInfo",
                "ConnectivityInfo"
            )
            | ("EdmondsBlossomInfo", "MatchingInfo")
            | (
                "KmpSearchInfo" | "RabinKarpInfo" | "BoyerMooreInfo",
                "StringSearchInfo"
            )
    )
}

pub fn net_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn ip_address_fields() -> Vec<Field> {
    vec![
        readonly_field("address", Type::Str),
        readonly_field("family", Type::Str),
    ]
}

pub fn socket_address_fields() -> Vec<Field> {
    vec![
        readonly_field("ip", Type::Named("IpAddress".to_string())),
        readonly_field("port", Type::U16),
    ]
}

pub fn udp_packet_fields() -> Vec<Field> {
    vec![
        field("data", Type::Array(Box::new(Type::U8))),
        field("address", Type::Str),
    ]
}

pub fn websocket_message_fields() -> Vec<Field> {
    vec![
        readonly_field("binary", Type::Bool),
        readonly_field("text", Type::Str),
        readonly_field("bytes", Type::Array(Box::new(Type::U8))),
    ]
}

pub fn temporal_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn instant_fields() -> Vec<Field> {
    vec![
        readonly_field("seconds", Type::I64),
        readonly_field("nanoseconds", Type::U32),
    ]
}

pub fn zoned_date_time_fields() -> Vec<Field> {
    vec![
        readonly_field("seconds", Type::I64),
        readonly_field("nanoseconds", Type::U32),
        readonly_field("timeZone", Type::Named("TimeZone".to_string())),
    ]
}

pub fn plain_date_fields() -> Vec<Field> {
    vec![
        readonly_field("year", Type::I32),
        readonly_field("month", Type::U32),
        readonly_field("day", Type::U32),
    ]
}

pub fn plain_time_fields() -> Vec<Field> {
    vec![
        readonly_field("hour", Type::U32),
        readonly_field("minute", Type::U32),
        readonly_field("second", Type::U32),
        readonly_field("nanosecond", Type::U32),
    ]
}

pub fn duration_fields() -> Vec<Field> {
    [
        "years",
        "months",
        "weeks",
        "days",
        "hours",
        "minutes",
        "seconds",
        "milliseconds",
        "microseconds",
        "nanoseconds",
    ]
    .into_iter()
    .map(|name| readonly_field(name, Type::I64))
    .collect()
}

pub fn plain_date_time_fields() -> Vec<Field> {
    vec![
        readonly_field("date", Type::Named("PlainDate".to_string())),
        readonly_field("time", Type::Named("PlainTime".to_string())),
    ]
}

pub fn plain_year_month_fields() -> Vec<Field> {
    vec![
        readonly_field("year", Type::I32),
        readonly_field("month", Type::U32),
    ]
}

pub fn plain_month_day_fields() -> Vec<Field> {
    vec![
        readonly_field("month", Type::U32),
        readonly_field("day", Type::U32),
    ]
}

pub fn notify_options_fields() -> Vec<Field> {
    vec![
        optional_field("icon", Type::Str),
        optional_field("urgency", Type::Named("NotifyUrgency".to_string())),
        optional_field("timeout", Type::U32),
    ]
}

pub fn font_info_fields() -> Vec<Field> {
    vec![
        readonly_field("name", Type::Str),
        readonly_field("style", Type::Str),
        readonly_field("path", Type::Str),
    ]
}

pub fn color_fields() -> Vec<Field> {
    vec![
        field("r", Type::U8),
        field("g", Type::U8),
        field("b", Type::U8),
        field("a", Type::U8),
    ]
}

pub fn fs_error_fields() -> Vec<Field> {
    vec![
        readonly_field("code", Type::Str),
        readonly_field("message", Type::Str),
    ]
}

pub fn file_stats_fields() -> Vec<Field> {
    vec![
        readonly_field("size", Type::U64),
        readonly_field("isFile", Type::Bool),
        readonly_field("isDirectory", Type::Bool),
        readonly_field("isSymlink", Type::Bool),
        readonly_field("modified", Type::U64),
        readonly_field("created", Type::U64),
        readonly_field("permissions", Type::U32),
    ]
}

pub fn dir_entry_fields() -> Vec<Field> {
    vec![
        readonly_field("name", Type::Str),
        readonly_field("type", Type::Named("DirEntryType".to_string())),
    ]
}

pub fn readdir_options_fields() -> Vec<Field> {
    vec![optional_field("recursive", Type::Bool)]
}

pub fn rm_options_fields() -> Vec<Field> {
    vec![
        optional_field("recursive", Type::Bool),
        optional_field("force", Type::Bool),
    ]
}

pub fn cp_options_fields() -> Vec<Field> {
    vec![
        optional_field("recursive", Type::Bool),
        optional_field("force", Type::Bool),
    ]
}

pub fn metadata_fields() -> Vec<Field> {
    vec![
        readonly_field("size", Type::U64),
        readonly_field("isFile", Type::Bool),
        readonly_field("isDirectory", Type::Bool),
        readonly_field("isSymlink", Type::Bool),
        readonly_field("modified", Type::U64),
        readonly_field("created", Type::U64),
        readonly_field("accessed", Type::U64),
        readonly_field("permissions", Type::U32),
        readonly_field("uid", Type::U32),
        readonly_field("gid", Type::U32),
        readonly_field("mode", Type::U32),
    ]
}

pub fn disk_usage_fields() -> Vec<Field> {
    vec![
        readonly_field("total", Type::U64),
        readonly_field("used", Type::U64),
        readonly_field("free", Type::U64),
    ]
}

pub fn search_options_fields() -> Vec<Field> {
    vec![
        optional_field("recursive", Type::Bool),
        optional_field("maxResults", Type::U32),
    ]
}

pub fn temp_dir_fields() -> Vec<Field> {
    vec![readonly_field("path", Type::Str)]
}

pub fn diff_result_fields() -> Vec<Field> {
    vec![
        readonly_field("added", Type::Array(Box::new(Type::Str))),
        readonly_field("removed", Type::Array(Box::new(Type::Str))),
        readonly_field("modified", Type::Array(Box::new(Type::Str))),
        readonly_field("unchanged", Type::Array(Box::new(Type::Str))),
    ]
}

pub fn fs_watcher_fields() -> Vec<Field> {
    vec![]
}

pub fn fs_watcher_event_fields() -> Vec<Field> {
    vec![
        readonly_field("kind", Type::Str),
        readonly_field("path", Type::Str),
    ]
}

pub fn stream_body_fields() -> Vec<Field> {
    vec![]
}

pub fn http_request_fields() -> Vec<Field> {
    vec![
        readonly_field("id", Type::Str),
        field("method", Type::Str),
        field("path", Type::Str),
        field("body", Type::Str),
        field("params", Type::Str),
        field("query", Type::Str),
        field("headers", Type::Str),
        field("cookies", Type::Str),
    ]
}

pub fn http_access_log_fields() -> Vec<Field> {
    vec![
        readonly_field("requestId", Type::Str),
        readonly_field("method", Type::Str),
        readonly_field("path", Type::Str),
        readonly_field("status", Type::Named("HttpStatus".to_string())),
        readonly_field("duration", Type::Named("Duration".to_string())),
    ]
}

pub fn http_request_abi_fields() -> Vec<Field> {
    let mut fields = http_request_fields();
    fields.push(optional_field("rawBody", Type::U8));
    fields.push(field("rawBodyLength", Type::Usize));
    fields.push(optional_field(
        "parts",
        Type::Named("MultipartPart".to_string()),
    ));
    fields.push(field("partsLength", Type::Usize));
    fields.push(field(
        "bodyStream",
        Type::Apply {
            name: "ReadableStream".to_string(),
            args: vec![Type::Array(Box::new(Type::U8))],
        },
    ));
    fields
}

pub fn http_response_fields() -> Vec<Field> {
    vec![
        field("statusCode", Type::Named("HttpStatus".to_string())),
        field(
            "body",
            Type::Union(vec![Type::Str, Type::Array(Box::new(Type::U8))]),
        ),
        optional_field("contentType", Type::Str),
        field("cookies", Type::Str),
    ]
}

pub fn http_cache_options_fields() -> Vec<Field> {
    vec![
        field("etag", Type::Str),
        optional_field("weak", Type::Bool),
        optional_field("lastModified", Type::Named("Instant".to_string())),
        optional_field("control", Type::Str),
    ]
}

pub fn http_reply_options_fields() -> Vec<Field> {
    vec![optional_field(
        "cache",
        Type::Named("HttpCacheOptions".to_string()),
    )]
}

pub fn fetch_policy_options_fields() -> Vec<Field> {
    vec![
        field("allow", Type::Array(Box::new(Type::Str))),
        field("denyPrivate", Type::Bool),
    ]
}

pub fn cors_options_fields() -> Vec<Field> {
    vec![
        field("origin", Type::Array(Box::new(Type::Str))),
        field("methods", Type::Array(Box::new(cors_method_type()))),
        field("allowedHeaders", Type::Array(Box::new(Type::Str))),
        field("exposedHeaders", Type::Array(Box::new(Type::Str))),
        field("credentials", Type::Bool),
        field("maxAge", Type::I32),
    ]
}

pub fn csrf_options_fields() -> Vec<Field> {
    vec![field("cookie", Type::Str), field("header", Type::Str)]
}

pub fn cors_options_abi_fields() -> Vec<Field> {
    vec![
        field("origin", Type::Array(Box::new(Type::Str))),
        field("methods", Type::Array(Box::new(Type::Str))),
        field("allowedHeaders", Type::Array(Box::new(Type::Str))),
        field("exposedHeaders", Type::Array(Box::new(Type::Str))),
        field("credentials", Type::Bool),
        field("maxAge", Type::I32),
    ]
}

pub fn server_options_fields() -> Vec<Field> {
    vec![
        optional_field("host", Type::Str),
        optional_field("requestTimeout", Type::Str),
        optional_field("maxConcurrentRequests", Type::I32),
        optional_field("shutdownTimeout", Type::Str),
        optional_field("headerTimeout", Type::Str),
        optional_field("maxHeaderSize", Type::Str),
        optional_field("maxConnections", Type::I32),
        optional_field("keepAlive", Type::Bool),
        optional_field("logger", Type::Bool),
        optional_field("tls", Type::Named("TlsOptions".to_string())),
    ]
}

pub fn text_body_fields() -> Vec<Field> {
    Vec::new()
}

pub fn bytes_body_fields() -> Vec<Field> {
    Vec::new()
}

pub fn http_header_fields() -> Vec<Field> {
    vec![field("name", Type::Str), field("value", Type::Str)]
}

/// What a `fetch` was asked to do, beyond the URL.
///
/// `timeout` and `maxBodySize` are written the way every other duration and
/// byte size in this module is, and read during compilation for the same
/// reason: the runtime must not carry a second parser that could disagree with
/// the checker. That is what makes the options a literal at the call.
///
/// The ABI shape still carries a method, because the verb fills it; see
/// [`fetch_options_abi_fields`].
pub fn fetch_options_fields() -> Vec<Field> {
    vec![
        optional_field(
            "headers",
            Type::Array(Box::new(Type::Named("HttpHeader".to_string()))),
        ),
        optional_field("body", Type::Str),
        optional_field(
            "bodyStream",
            Type::Apply {
                name: "ReadableStream".to_string(),
                args: vec![Type::Array(Box::new(Type::U8))],
            },
        ),
        optional_field("timeout", Type::Str),
        optional_field("maxBodySize", Type::Str),
        optional_field("redirects", Type::I32),
        optional_field("decompress", Type::Bool),
    ]
}

pub fn fetch_options_abi_fields() -> Vec<Field> {
    vec![
        optional_field("method", Type::Str),
        optional_field(
            "headers",
            Type::Array(Box::new(Type::Named("HttpHeader".to_string()))),
        ),
        optional_field("body", Type::Str),
        field("timeoutMillis", Type::I64),
        field("maxBodyBytes", Type::I64),
        field("redirects", Type::I64),
        optional_field(
            "bodyStream",
            Type::Apply {
                name: "ReadableStream".to_string(),
                args: vec![Type::Array(Box::new(Type::U8))],
            },
        ),
        field("decompress", Type::I64),
    ]
}

pub fn multipart_body_fields() -> Vec<Field> {
    Vec::new()
}

pub fn multipart_part_fields() -> Vec<Field> {
    vec![
        readonly_field("name", Type::Str),
        readonly_field("fileName", Type::Optional(Box::new(Type::Str))),
        readonly_field("contentType", Type::Optional(Box::new(Type::Str))),
        readonly_field("bytes", Type::Array(Box::new(Type::U8))),
    ]
}

pub fn tls_options_fields() -> Vec<Field> {
    vec![field("cert", Type::Str), field("key", Type::Str)]
}

pub fn rate_limit_fields() -> Vec<Field> {
    vec![
        field("max", Type::I32),
        field("timeWindow", Type::Str),
        optional_field("globalMax", Type::I32),
        optional_field("ban", Type::I32),
        optional_field("banWindow", Type::Str),
    ]
}

pub fn cookie_options_fields() -> Vec<Field> {
    vec![
        optional_field("domain", Type::Str),
        optional_field("path", Type::Str),
        optional_field("maxAge", Type::I64),
        optional_field("secure", Type::Bool),
        optional_field("httpOnly", Type::Bool),
        optional_field("sameSite", cookie_same_site_type()),
    ]
}

pub fn http_cookie_fields() -> Vec<Field> {
    vec![
        field("name", Type::Str),
        field("value", Type::Str),
        optional_field("options", Type::Named("CookieOptions".to_string())),
    ]
}

pub fn cookie_options_abi_fields() -> Vec<Field> {
    vec![
        optional_field("domain", Type::Str),
        optional_field("path", Type::Str),
        optional_field("maxAge", Type::I64),
        optional_field("secure", Type::Bool),
        optional_field("httpOnly", Type::Bool),
        optional_field("sameSite", Type::Str),
    ]
}

pub fn ambient_builtin_interfaces() -> HashMap<String, Vec<Field>> {
    [
        (
            "ValidationIssue".to_string(),
            vec![
                readonly_field("target", Type::Str),
                readonly_field("validator", Type::Str),
                readonly_field("message", Type::Str),
            ],
        ),
        (
            "ValidationError".to_string(),
            vec![
                readonly_field(
                    "name",
                    Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(
                        "ValidationError".to_string(),
                    )))),
                ),
                readonly_field("message", Type::Str),
                readonly_field(
                    "issues",
                    Type::Readonly(Box::new(Type::Array(Box::new(Type::Named(
                        "ValidationIssue".to_string(),
                    ))))),
                ),
            ],
        ),
        (
            "JsonError".to_string(),
            vec![
                readonly_field("code", Type::Str),
                readonly_field("message", Type::Str),
                readonly_field("path", Type::Str),
            ],
        ),
        (
            "MemoryInfo".to_string(),
            vec![readonly_field("rss", Type::U64)],
        ),
        ("ZipError".to_string(), zip_error_fields()),
        (
            "CpuUsage".to_string(),
            vec![
                readonly_field("user", Type::U64),
                readonly_field("system", Type::U64),
            ],
        ),
    ]
    .into_iter()
    .collect()
}

pub fn builtin_interfaces() -> HashMap<String, Vec<Field>> {
    let mut interfaces = ambient_builtin_interfaces();
    interfaces.insert("HttpRequest".to_string(), http_request_abi_fields());
    interfaces.insert("HttpAccessLog".to_string(), http_access_log_fields());
    interfaces.insert("HttpResponse".to_string(), http_response_fields());
    interfaces.insert("HttpCacheOptions".to_string(), http_cache_options_fields());
    interfaces.insert("HttpReplyOptions".to_string(), http_reply_options_fields());
    // The ABI variants: this map is what code generation lays structs out
    // from, and a struct crossing into the runtime must match what the runtime
    // reads. See `cors_options_abi_fields`.
    interfaces.insert("ServerOptions".to_string(), server_options_fields());
    interfaces.insert("RateLimit".to_string(), rate_limit_fields());
    interfaces.insert("TlsOptions".to_string(), tls_options_fields());
    interfaces.insert("TextBody".to_string(), text_body_fields());
    interfaces.insert("BytesBody".to_string(), bytes_body_fields());
    interfaces.insert("StreamBody".to_string(), stream_body_fields());
    interfaces.insert("MultipartBody".to_string(), multipart_body_fields());
    interfaces.insert("MultipartPart".to_string(), multipart_part_fields());
    interfaces.insert("HttpHeader".to_string(), http_header_fields());
    interfaces.insert("FetchOptions".to_string(), fetch_options_abi_fields());
    interfaces.insert("CorsOptions".to_string(), cors_options_abi_fields());
    interfaces.insert("CsrfOptions".to_string(), csrf_options_fields());
    interfaces.insert(
        "FetchPolicyOptions".to_string(),
        fetch_policy_options_fields(),
    );
    interfaces.insert("CookieOptions".to_string(), cookie_options_abi_fields());
    interfaces.insert("HttpCookie".to_string(), http_cookie_fields());
    interfaces.insert("UserInfo".to_string(), user_info_fields());
    interfaces.insert("ProcessError".to_string(), process_error_fields());
    interfaces.insert("ChildProcess".to_string(), child_process_fields());
    interfaces.insert("CpuInfo".to_string(), cpu_info_fields());
    interfaces.insert("BatteryInfo".to_string(), battery_info_fields());
    interfaces.insert("NetworkInterface".to_string(), network_interface_fields());
    interfaces.insert("HttpError".to_string(), http_error_fields());
    interfaces.insert("StreamError".to_string(), stream_error_fields());
    interfaces.insert("OsError".to_string(), os_error_fields());
    interfaces.insert("ArchiveError".to_string(), archive_error_fields());
    interfaces.insert("ArchiveEntry".to_string(), archive_entry_fields());
    interfaces.insert("ArchiveSummary".to_string(), archive_summary_fields());
    interfaces.insert("PackOptions".to_string(), pack_options_fields());
    interfaces.insert("UnpackOptions".to_string(), unpack_options_fields());
    interfaces.insert("CompressError".to_string(), compress_error_fields());
    interfaces.insert("CompressOptions".to_string(), compress_options_fields());
    interfaces.insert("DecompressOptions".to_string(), decompress_options_fields());
    interfaces.insert("DbError".to_string(), db_error_fields());
    interfaces.insert("Page".to_string(), db_page_fields());
    interfaces.insert("SimdError".to_string(), simd_error_fields());
    interfaces.insert("EncodingError".to_string(), encoding_error_fields());
    interfaces.insert("CryptoError".to_string(), crypto_error_fields());
    interfaces.insert("MemoryError".to_string(), memory_error_fields());
    interfaces.insert("SortInfo".to_string(), sort_info_fields());
    interfaces.insert("BubbleSortInfo".to_string(), bubble_sort_info_fields());
    interfaces.insert("SearchInfo".to_string(), search_info_fields());
    interfaces.insert(
        "InterpolationSearchInfo".to_string(),
        interpolation_search_info_fields(),
    );
    interfaces.insert(
        "ExponentialSearchInfo".to_string(),
        exponential_search_info_fields(),
    );
    interfaces.insert("SelectionInfo".to_string(), selection_info_fields());
    interfaces.insert("TraversalInfo".to_string(), traversal_info_fields());
    interfaces.insert("IddfsInfo".to_string(), iddfs_info_fields());
    interfaces.insert("ShortestPathInfo".to_string(), shortest_path_info_fields());
    interfaces.insert("DijkstraInfo".to_string(), dijkstra_info_fields());
    interfaces.insert("AStarInfo".to_string(), a_star_info_fields());
    interfaces.insert("BellmanFordInfo".to_string(), bellman_ford_info_fields());
    interfaces.insert("SpfaInfo".to_string(), spfa_info_fields());
    interfaces.insert("JohnsonInfo".to_string(), johnson_info_fields());
    interfaces.insert(
        "FloydWarshallInfo".to_string(),
        floyd_warshall_info_fields(),
    );
    interfaces.insert("SpanningTreeInfo".to_string(), spanning_tree_info_fields());
    interfaces.insert("FlowInfo".to_string(), flow_info_fields());
    interfaces.insert(
        "TopologicalSortInfo".to_string(),
        topological_sort_info_fields(),
    );
    interfaces.insert(
        "TopologicalSortKahnInfo".to_string(),
        topological_sort_kahn_info_fields(),
    );
    interfaces.insert(
        "TopologicalSortDfsInfo".to_string(),
        topological_sort_dfs_info_fields(),
    );
    interfaces.insert("SccInfo".to_string(), scc_info_fields());
    interfaces.insert("TarjanSccInfo".to_string(), tarjan_scc_info_fields());
    interfaces.insert("KosarajuSccInfo".to_string(), kosaraju_scc_info_fields());
    interfaces.insert("EulerianInfo".to_string(), eulerian_info_fields());
    interfaces.insert(
        "FleuryEulerianInfo".to_string(),
        fleury_eulerian_info_fields(),
    );
    interfaces.insert(
        "HierholzerEulerianInfo".to_string(),
        hierholzer_eulerian_info_fields(),
    );
    interfaces.insert("ConnectivityInfo".to_string(), connectivity_info_fields());
    interfaces.insert("FindBridgesInfo".to_string(), find_bridges_info_fields());
    interfaces.insert(
        "FindCutVerticesInfo".to_string(),
        find_cut_vertices_info_fields(),
    );
    interfaces.insert(
        "BiconnectedComponentsInfo".to_string(),
        biconnected_components_info_fields(),
    );
    interfaces.insert("MatchingInfo".to_string(), matching_info_fields());
    interfaces.insert(
        "EdmondsBlossomInfo".to_string(),
        edmonds_blossom_info_fields(),
    );
    interfaces.insert("HungarianInfo".to_string(), hungarian_info_fields());
    interfaces.insert("StringSearchInfo".to_string(), string_search_info_fields());
    interfaces.insert("KmpSearchInfo".to_string(), kmp_search_info_fields());
    interfaces.insert("ZAlgorithmInfo".to_string(), z_algorithm_info_fields());
    interfaces.insert("RabinKarpInfo".to_string(), rabin_karp_info_fields());
    interfaces.insert("BoyerMooreInfo".to_string(), boyer_moore_info_fields());
    interfaces.insert("AhoCorasickInfo".to_string(), aho_corasick_info_fields());
    interfaces.insert("EditDistanceInfo".to_string(), edit_distance_info_fields());
    interfaces.insert("SuffixArrayInfo".to_string(), suffix_array_info_fields());
    interfaces.insert("EdmondsKarpInfo".to_string(), edmonds_karp_info_fields());
    interfaces.insert("DinicInfo".to_string(), dinic_info_fields());
    interfaces.insert("PushRelabelInfo".to_string(), push_relabel_info_fields());
    interfaces.insert("MinCutInfo".to_string(), min_cut_info_fields());
    interfaces.insert(
        "MinCostMaxFlowInfo".to_string(),
        min_cost_max_flow_info_fields(),
    );
    interfaces.insert("KruskalInfo".to_string(), kruskal_info_fields());
    interfaces.insert("PrimInfo".to_string(), prim_info_fields());
    interfaces.insert("BoruvkaInfo".to_string(), boruvka_info_fields());
    interfaces.insert("SteinerTreeInfo".to_string(), steiner_tree_info_fields());
    interfaces.insert("NetError".to_string(), net_error_fields());
    interfaces.insert("IpAddress".to_string(), ip_address_fields());
    interfaces.insert("SocketAddress".to_string(), socket_address_fields());
    interfaces.insert("UdpPacket".to_string(), udp_packet_fields());
    interfaces.insert("WebSocketMessage".to_string(), websocket_message_fields());
    interfaces.insert("TemporalError".to_string(), temporal_error_fields());
    interfaces.insert("Instant".to_string(), instant_fields());
    interfaces.insert("ZonedDateTime".to_string(), zoned_date_time_fields());
    interfaces.insert("PlainDate".to_string(), plain_date_fields());
    interfaces.insert("PlainTime".to_string(), plain_time_fields());
    interfaces.insert("Duration".to_string(), duration_fields());
    interfaces.insert("PlainDateTime".to_string(), plain_date_time_fields());
    interfaces.insert("PlainYearMonth".to_string(), plain_year_month_fields());
    interfaces.insert("PlainMonthDay".to_string(), plain_month_day_fields());
    interfaces.insert("NotifyOptions".to_string(), notify_options_fields());
    interfaces.insert("FontInfo".to_string(), font_info_fields());
    interfaces.insert("Color".to_string(), color_fields());
    interfaces.insert("FsError".to_string(), fs_error_fields());
    interfaces.insert("FileStats".to_string(), file_stats_fields());
    interfaces.insert("DirEntry".to_string(), dir_entry_fields());
    interfaces.insert("ReaddirOptions".to_string(), readdir_options_fields());
    interfaces.insert("RmOptions".to_string(), rm_options_fields());
    interfaces.insert("CpOptions".to_string(), cp_options_fields());
    interfaces.insert("Metadata".to_string(), metadata_fields());
    interfaces.insert("DiskUsage".to_string(), disk_usage_fields());
    interfaces.insert("SearchOptions".to_string(), search_options_fields());
    interfaces.insert("TempDir".to_string(), temp_dir_fields());
    interfaces.insert("DiffResult".to_string(), diff_result_fields());
    interfaces.insert("FSWatcher".to_string(), fs_watcher_fields());
    interfaces.insert("FSWatcherEvent".to_string(), fs_watcher_event_fields());
    for (name, fields) in crate::algorithm::algorithm_interface_fields() {
        interfaces.insert(name, fields);
    }
    interfaces.insert("TyrusTestAssertion".to_string(), Vec::new());
    interfaces
}

/// Whether `name` is a payload-free enum a native module exports. Such an
/// enum can reach a program as the field type of an imported interface even
/// when the program never imports the enum itself.
pub fn is_builtin_unit_enum(name: &str) -> bool {
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NAMES
        .get_or_init(|| {
            builtin_enums()
                .into_iter()
                .filter(|(_, resolved)| !resolved.is_algebraic)
                .map(|(name, _)| name)
                .collect()
        })
        .iter()
        .any(|known| known == name)
}

pub fn builtin_enums() -> HashMap<String, ResolvedEnum> {
    [
        (
            "DirEntryType".to_string(),
            crate::module_loader::dir_entry_type_enum(),
        ),
        (
            "FileMode".to_string(),
            crate::module_loader::file_mode_enum(),
        ),
        (
            "HttpStatus".to_string(),
            crate::module_loader::http_status_enum(),
        ),
        (
            "ArchiveEntryKind".to_string(),
            crate::archive::archive_entry_kind_enum(),
        ),
        (
            "ArchiveFormat".to_string(),
            crate::archive::archive_format_enum(),
        ),
        (
            "CompressFormat".to_string(),
            crate::compress::compress_format_enum(),
        ),
        ("OsPlatform".to_string(), crate::os::os_platform_enum()),
        ("OsArch".to_string(), crate::os::os_arch_enum()),
        (
            "NotifyUrgency".to_string(),
            crate::os::notify_urgency_enum(),
        ),
        (
            "SystemCapability".to_string(),
            crate::os::system_capability_enum(),
        ),
        ("TimeZone".to_string(), crate::temporal::time_zone_enum()),
    ]
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contains_literal_union(ty: &Type) -> bool {
        match ty {
            Type::Union(variants) => {
                !variants.is_empty()
                    && variants
                        .iter()
                        .all(|variant| matches!(variant, Type::Lit(_)))
            }
            Type::Array(element) | Type::Optional(element) => contains_literal_union(element),
            Type::Ref(inner) | Type::MutRef(inner) | Type::Readonly(inner) => {
                contains_literal_union(inner)
            }
            _ => false,
        }
    }

    #[test]
    fn no_interface_codegen_lays_out_carries_a_literal_union() {
        for (name, fields) in builtin_interfaces() {
            for field in fields {
                assert!(
                    !contains_literal_union(&field.ty),
                    "`{name}.{}` reaches codegen as a union; give the interface an ABI \
                     variant whose field uses the type the union's literals belong to",
                    field.name
                );
            }
        }
    }

    #[test]
    fn abi_interface_variants_match_their_checker_fields() {
        for (checker, abi) in [
            (cors_options_fields(), cors_options_abi_fields()),
            (cookie_options_fields(), cookie_options_abi_fields()),
        ] {
            let names = |fields: &[Field]| {
                fields
                    .iter()
                    .map(|field| (field.name.clone(), field.optional, field.readonly))
                    .collect::<Vec<_>>()
            };
            assert_eq!(names(&checker), names(&abi));
        }
    }
}
