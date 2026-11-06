use super::*;
use std::collections::HashMap;
use std::ptr;
use tyrus_string::TyrusString;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

pub const GRAPH_TAG_SIGNED: u8 = 1;
pub const GRAPH_TAG_UNSIGNED: u8 = 2;
pub const GRAPH_TAG_FLOAT: u8 = 3;
pub const GRAPH_TAG_BOOL: u8 = 4;
pub const GRAPH_TAG_STRING: u8 = 5;

fn allocation_failed() -> ! {
    unsafe { crate::tyrus_panic(c"tyrus runtime error: Graph allocation failed".as_ptr()) }
}

pub struct TyrusGraph {
    node_index: HashMap<Vec<u8>, usize>,
    nodes: Vec<Vec<u8>>,
    adjacency: Vec<Vec<Edge>>,
    node_size: usize,
    weight_size: usize,
    node_tag: u8,
    weight_tag: u8,
}

pub(crate) struct Edge {
    pub(crate) to: usize,
    pub(crate) weight: Vec<u8>,
}

impl TyrusGraph {
    pub(crate) fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn adjacency(&self) -> &[Vec<Edge>] {
        &self.adjacency
    }

    pub(crate) fn weight_as_i32(&self, weight: &[u8]) -> i32 {
        let bytes: [u8; 4] = weight
            .try_into()
            .unwrap_or_else(|_| panic!("tyrus runtime error: Graph weight is not an i32"));
        match self.weight_tag {
            GRAPH_TAG_SIGNED => i32::from_le_bytes(bytes),
            GRAPH_TAG_UNSIGNED => u32::from_le_bytes(bytes) as i32,
            _ => panic!("tyrus runtime error: Graph weight is not an i32"),
        }
    }

    unsafe fn canonical_node_key(&self, node: *const u8) -> Option<Vec<u8>> {
        unsafe {
            if node.is_null() {
                return None;
            }
            if self.node_tag == GRAPH_TAG_STRING {
                let stored = &*(node.cast::<TyrusString>());
                text(stored).ok().map(String::into_bytes)
            } else {
                Some(std::slice::from_raw_parts(node, self.node_size).to_vec())
            }
        }
    }

    fn node_id(&self, key: &[u8]) -> Option<usize> {
        self.node_index.get(key).copied()
    }

    fn ensure_node(&mut self, key: Vec<u8>) -> usize {
        if let Some(&id) = self.node_index.get(&key) {
            return id;
        }
        let id = self.nodes.len();
        self.node_index.insert(key.clone(), id);
        self.nodes.push(key);
        self.adjacency.push(Vec::new());
        id
    }

    unsafe fn collect_nodes(&self, output: *mut TyrusArray, ids: Vec<usize>) {
        unsafe {
            let Some(output) = output.as_mut() else {
                return;
            };
            let length = ids.len();
            let stride = if self.node_tag == GRAPH_TAG_STRING {
                std::mem::size_of::<TyrusString>()
            } else {
                self.node_size
            };
            let data = if length == 0 {
                ptr::null_mut()
            } else {
                malloc(length * stride).cast::<u8>()
            };
            if length > 0 && data.is_null() {
                allocation_failed();
            }
            for (index, id) in ids.iter().enumerate() {
                let slot = data.add(index * stride);
                if self.node_tag == GRAPH_TAG_STRING {
                    let text = std::str::from_utf8(self.nodes[*id].as_slice()).unwrap_or_default();
                    *(slot.cast::<TyrusString>()) = crate::strings::alloc_string(text);
                } else {
                    ptr::copy_nonoverlapping(self.nodes[*id].as_ptr(), slot, self.node_size);
                }
            }
            *output = TyrusArray {
                data,
                length,
                capacity: length,
            };
        }
    }

    fn append_value(&self, out: &mut String, bytes: &[u8], tag: u8) {
        match tag {
            GRAPH_TAG_STRING => {
                let value = std::str::from_utf8(bytes).unwrap_or("");
                out.push_str(value);
            }
            GRAPH_TAG_BOOL => {
                out.push_str(if bytes.first().copied().unwrap_or(0) != 0 {
                    "true"
                } else {
                    "false"
                });
            }
            GRAPH_TAG_FLOAT if bytes.len() == 4 => {
                let value = f32::from_le_bytes(bytes.try_into().unwrap()) as f64;
                append_f64(out, value);
            }
            GRAPH_TAG_FLOAT => {
                let value = f64::from_le_bytes(bytes.try_into().unwrap());
                append_f64(out, value);
            }
            GRAPH_TAG_UNSIGNED => {
                let value = widen_unsigned(bytes);
                out.push_str(&value.to_string());
            }
            _ => {
                let value = widen_signed(bytes);
                out.push_str(&value.to_string());
            }
        }
    }

    fn node_label(&self, out: &mut String, node: &[u8]) {
        let mut rendered = String::new();
        if self.node_tag == GRAPH_TAG_STRING {
            rendered.push_str(std::str::from_utf8(node).unwrap_or(""));
        } else {
            self.append_value(&mut rendered, node, self.node_tag);
        }
        out.push('"');
        for ch in rendered.chars() {
            if ch == '"' || ch == '\\' {
                out.push('\\');
            }
            out.push(ch);
        }
        out.push('"');
    }
}

fn widen_signed(bytes: &[u8]) -> i128 {
    match bytes.len() {
        1 => i8::from_le_bytes(bytes.try_into().unwrap()) as i128,
        2 => i16::from_le_bytes(bytes.try_into().unwrap()) as i128,
        4 => i32::from_le_bytes(bytes.try_into().unwrap()) as i128,
        8 => i64::from_le_bytes(bytes.try_into().unwrap()) as i128,
        16 => i128::from_le_bytes(bytes.try_into().unwrap()),
        _ => 0,
    }
}

fn widen_unsigned(bytes: &[u8]) -> u128 {
    match bytes.len() {
        1 => u8::from_le_bytes(bytes.try_into().unwrap()) as u128,
        2 => u16::from_le_bytes(bytes.try_into().unwrap()) as u128,
        4 => u32::from_le_bytes(bytes.try_into().unwrap()) as u128,
        8 => u64::from_le_bytes(bytes.try_into().unwrap()) as u128,
        16 => u128::from_le_bytes(bytes.try_into().unwrap()),
        _ => 0,
    }
}

fn append_f64(out: &mut String, value: f64) {
    if value.is_finite() && value == value.trunc() {
        out.push_str(&format!("{value:.1}"));
    } else {
        out.push_str(&format!("{value}"));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_new(
    node_size: usize,
    weight_size: usize,
    node_tag: u8,
    weight_tag: u8,
) -> *mut TyrusGraph {
    let graph = Box::new(TyrusGraph {
        node_index: HashMap::new(),
        nodes: Vec::new(),
        adjacency: Vec::new(),
        node_size,
        weight_size,
        node_tag,
        weight_tag,
    });
    Box::into_raw(graph)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_free(graph: *mut TyrusGraph) {
    unsafe {
        if !graph.is_null() {
            drop(Box::from_raw(graph));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_add_node(graph: *mut TyrusGraph, node: *const u8) {
    unsafe {
        let graph = &mut *graph;
        let Some(key) = graph.canonical_node_key(node) else {
            return;
        };
        if graph.node_index.contains_key(&key) {
            return;
        }
        let id = graph.nodes.len();
        graph.node_index.insert(key.clone(), id);
        graph.nodes.push(key);
        graph.adjacency.push(Vec::new());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_add_edge(
    graph: *mut TyrusGraph,
    source: *const u8,
    target: *const u8,
    weight: *const u8,
) {
    unsafe {
        let graph = &mut *graph;
        let Some(source_key) = graph.canonical_node_key(source) else {
            return;
        };
        let Some(target_key) = graph.canonical_node_key(target) else {
            return;
        };
        let weight_bytes = std::slice::from_raw_parts(weight, graph.weight_size).to_vec();
        let source_id = graph.ensure_node(source_key);
        let target_id = graph.ensure_node(target_key);
        if let Some(edge) = graph.adjacency[source_id]
            .iter_mut()
            .find(|edge| edge.to == target_id)
        {
            edge.weight = weight_bytes;
        } else {
            graph.adjacency[source_id].push(Edge {
                to: target_id,
                weight: weight_bytes,
            });
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_remove_node(graph: *mut TyrusGraph, node: *const u8) -> bool {
    unsafe {
        let graph = &mut *graph;
        let Some(key) = graph.canonical_node_key(node) else {
            return false;
        };
        let Some(removed_id) = graph.node_index.remove(&key) else {
            return false;
        };
        for adjacency in &mut graph.adjacency {
            adjacency.retain(|edge| edge.to != removed_id);
        }
        graph.nodes.remove(removed_id);
        graph.adjacency.remove(removed_id);
        for id in graph.node_index.values_mut() {
            if *id > removed_id {
                *id -= 1;
            }
        }
        for adjacency in &mut graph.adjacency {
            for edge in adjacency.iter_mut() {
                if edge.to > removed_id {
                    edge.to -= 1;
                }
            }
        }
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_remove_edge(
    graph: *mut TyrusGraph,
    source: *const u8,
    target: *const u8,
) -> bool {
    unsafe {
        let graph = &mut *graph;
        let Some(source_key) = graph.canonical_node_key(source) else {
            return false;
        };
        let Some(target_key) = graph.canonical_node_key(target) else {
            return false;
        };
        let Some(source_id) = graph.node_id(&source_key) else {
            return false;
        };
        let Some(target_id) = graph.node_id(&target_key) else {
            return false;
        };
        let Some(position) = graph.adjacency[source_id]
            .iter()
            .position(|edge| edge.to == target_id)
        else {
            return false;
        };
        graph.adjacency[source_id].swap_remove(position);
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_has_node(graph: *const TyrusGraph, node: *const u8) -> bool {
    unsafe {
        let graph = &*graph;
        graph
            .canonical_node_key(node)
            .is_some_and(|key| graph.node_id(&key).is_some())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_has_edge(
    graph: *const TyrusGraph,
    source: *const u8,
    target: *const u8,
) -> bool {
    unsafe {
        let graph = &*graph;
        let (Some(source_id), Some(target_id)) = (
            graph
                .canonical_node_key(source)
                .and_then(|key| graph.node_id(&key)),
            graph
                .canonical_node_key(target)
                .and_then(|key| graph.node_id(&key)),
        ) else {
            return false;
        };
        graph.adjacency[source_id]
            .iter()
            .any(|edge| edge.to == target_id)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_edge(
    graph: *const TyrusGraph,
    source: *const u8,
    target: *const u8,
) -> *mut u8 {
    unsafe {
        let graph = &*graph;
        let (Some(source_id), Some(target_id)) = (
            graph
                .canonical_node_key(source)
                .and_then(|key| graph.node_id(&key)),
            graph
                .canonical_node_key(target)
                .and_then(|key| graph.node_id(&key)),
        ) else {
            return ptr::null_mut();
        };
        let Some(edge) = graph.adjacency[source_id]
            .iter()
            .find(|edge| edge.to == target_id)
        else {
            return ptr::null_mut();
        };
        let storage = malloc(graph.weight_size.max(1)).cast::<u8>();
        if storage.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(edge.weight.as_ptr(), storage, graph.weight_size);
        storage
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_neighbors(
    graph: *const TyrusGraph,
    node: *const u8,
    output: *mut TyrusArray,
) {
    unsafe {
        let graph = &*graph;
        let Some(node_id) = graph
            .canonical_node_key(node)
            .and_then(|key| graph.node_id(&key))
        else {
            if let Some(output) = output.as_mut() {
                *output = TyrusArray {
                    data: ptr::null_mut(),
                    length: 0,
                    capacity: 0,
                };
            }
            return;
        };
        let targets: Vec<usize> = graph.adjacency[node_id]
            .iter()
            .map(|edge| edge.to)
            .collect();
        graph.collect_nodes(output, targets);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_degree(graph: *const TyrusGraph, node: *const u8) -> usize {
    unsafe {
        let graph = &*graph;
        graph
            .canonical_node_key(node)
            .and_then(|key| graph.node_id(&key))
            .map_or(0, |id| graph.adjacency[id].len())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_node_count(graph: *const TyrusGraph) -> usize {
    unsafe { (*graph).nodes.len() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_edge_count(graph: *const TyrusGraph) -> usize {
    unsafe {
        let graph = &*graph;
        graph.adjacency.iter().map(std::vec::Vec::len).sum()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_is_empty(graph: *const TyrusGraph) -> bool {
    unsafe { (*graph).nodes.is_empty() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_nodes(graph: *const TyrusGraph, output: *mut TyrusArray) {
    unsafe {
        let graph = &*graph;
        graph.collect_nodes(output, (0..graph.nodes.len()).collect());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_edges_sources(
    graph: *const TyrusGraph,
    output: *mut TyrusArray,
) {
    unsafe {
        let graph = &*graph;
        let sources: Vec<usize> = graph
            .adjacency
            .iter()
            .enumerate()
            .flat_map(|(source, edges)| edges.iter().map(move |_| source))
            .collect();
        graph.collect_nodes(output, sources);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_edges_targets(
    graph: *const TyrusGraph,
    output: *mut TyrusArray,
) {
    unsafe {
        let graph = &*graph;
        let targets: Vec<usize> = graph
            .adjacency
            .iter()
            .flat_map(|edges| edges.iter().map(|edge| edge.to))
            .collect();
        graph.collect_nodes(output, targets);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_edges_weights(
    graph: *const TyrusGraph,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(graph) = graph.as_ref() else {
            return;
        };
        let Some(output) = output.as_mut() else {
            return;
        };
        let total: usize = graph.adjacency.iter().map(std::vec::Vec::len).sum();
        let bytes = total
            .checked_mul(graph.weight_size)
            .unwrap_or_else(|| allocation_failed());
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes).cast::<u8>()
        };
        if bytes != 0 && data.is_null() {
            allocation_failed();
        }
        let mut cursor = 0usize;
        for edges in &graph.adjacency {
            for edge in edges {
                if bytes != 0 {
                    ptr::copy_nonoverlapping(
                        edge.weight.as_ptr(),
                        data.add(cursor),
                        graph.weight_size,
                    );
                }
                cursor += graph.weight_size;
            }
        }
        *output = TyrusArray {
            data,
            length: total,
            capacity: total,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_reverse(graph: *const TyrusGraph) -> *mut TyrusGraph {
    unsafe {
        let graph = &*graph;
        let mut reversed = Box::new(TyrusGraph {
            node_index: HashMap::new(),
            nodes: graph.nodes.clone(),
            adjacency: (0..graph.nodes.len()).map(|_| Vec::new()).collect(),
            node_size: graph.node_size,
            weight_size: graph.weight_size,
            node_tag: graph.node_tag,
            weight_tag: graph.weight_tag,
        });
        reversed.node_index.clone_from(&graph.node_index);
        for (source, edges) in graph.adjacency.iter().enumerate() {
            for edge in edges {
                reversed.adjacency[edge.to].push(Edge {
                    to: source,
                    weight: edge.weight.clone(),
                });
            }
        }
        Box::into_raw(reversed)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_to_dot(graph: *const TyrusGraph) -> TyrusString {
    unsafe {
        let graph = &*graph;
        let mut out = String::from("digraph {\n");
        for (source, edges) in graph.adjacency.iter().enumerate() {
            for edge in edges {
                out.push_str("    ");
                graph.node_label(&mut out, &graph.nodes[source]);
                out.push_str(" -> ");
                graph.node_label(&mut out, &graph.nodes[edge.to]);
                out.push_str(" [label=\"");
                graph.append_value(&mut out, &edge.weight, graph.weight_tag);
                out.push_str("\"];\n");
            }
        }
        out.push_str("}\n");
        crate::strings::alloc_string(&out)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_graph_to_string(graph: *const TyrusGraph) -> TyrusString {
    unsafe {
        let graph = &*graph;
        let mut out = String::from("Graph {");
        let mut first = true;
        for (source, edges) in graph.adjacency.iter().enumerate() {
            for edge in edges {
                if !first {
                    out.push(',');
                }
                first = false;
                out.push(' ');
                graph.node_label(&mut out, &graph.nodes[source]);
                out.push_str(" -> ");
                graph.node_label(&mut out, &graph.nodes[edge.to]);
                out.push_str(" [label=\"");
                graph.append_value(&mut out, &edge.weight, graph.weight_tag);
                out.push_str("\"]");
            }
        }
        if !first {
            out.push(' ');
        }
        out.push('}');
        crate::strings::alloc_string(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" {
        fn free(ptr: *mut c_void);
    }

    unsafe fn node_ptr(value: &mut i32) -> *const u8 {
        value as *mut i32 as *const u8
    }

    unsafe fn weight_ptr<T: Copy>(value: &mut T) -> *const u8 {
        value as *mut T as *const u8
    }

    #[test]
    fn add_edge_creates_nodes_and_replaces_weights() {
        let graph = unsafe { tyrus_graph_new(4, 4, GRAPH_TAG_SIGNED, GRAPH_TAG_SIGNED) };
        let mut a = 1i32;
        let mut b = 2i32;
        let mut w = 10i32;
        unsafe {
            tyrus_graph_add_edge(
                graph,
                node_ptr(&mut a),
                node_ptr(&mut b),
                weight_ptr(&mut w),
            );
            assert!(tyrus_graph_has_node(graph, node_ptr(&mut a)));
            assert!(tyrus_graph_has_node(graph, node_ptr(&mut b)));
            assert_eq!(tyrus_graph_node_count(graph), 2);
            assert_eq!(tyrus_graph_edge_count(graph), 1);
            assert_eq!(tyrus_graph_degree(graph, node_ptr(&mut a)), 1);
            assert_eq!(tyrus_graph_degree(graph, node_ptr(&mut b)), 0);

            let edge = tyrus_graph_edge(graph, node_ptr(&mut a), node_ptr(&mut b));
            assert_eq!(*(edge.cast::<i32>()), 10);
            free(edge.cast());

            let mut w2 = 99i32;
            tyrus_graph_add_edge(
                graph,
                node_ptr(&mut a),
                node_ptr(&mut b),
                weight_ptr(&mut w2),
            );
            assert_eq!(
                tyrus_graph_edge_count(graph),
                1,
                "one edge per ordered pair"
            );
            let edge = tyrus_graph_edge(graph, node_ptr(&mut a), node_ptr(&mut b));
            assert_eq!(*(edge.cast::<i32>()), 99);
            free(edge.cast());

            assert!(tyrus_graph_remove_edge(
                graph,
                node_ptr(&mut a),
                node_ptr(&mut b)
            ));
            assert_eq!(tyrus_graph_edge_count(graph), 0);
            assert!(!tyrus_graph_has_edge(
                graph,
                node_ptr(&mut a),
                node_ptr(&mut b)
            ));

            let empty = tyrus_graph_edge(graph, node_ptr(&mut a), node_ptr(&mut b));
            assert!(empty.is_null(), "absent edge returns null `W?`");

            tyrus_graph_free(graph);
        }
    }

    #[test]
    fn remove_node_drops_incident_edges_and_renumbers() {
        let graph = unsafe { tyrus_graph_new(4, 4, GRAPH_TAG_SIGNED, GRAPH_TAG_SIGNED) };
        let mut a = 1i32;
        let mut b = 2i32;
        let mut c = 3i32;
        let mut w = 5i32;
        unsafe {
            tyrus_graph_add_edge(
                graph,
                node_ptr(&mut a),
                node_ptr(&mut b),
                weight_ptr(&mut w),
            );
            tyrus_graph_add_edge(
                graph,
                node_ptr(&mut b),
                node_ptr(&mut a),
                weight_ptr(&mut w),
            );
            tyrus_graph_add_edge(
                graph,
                node_ptr(&mut b),
                node_ptr(&mut c),
                weight_ptr(&mut w),
            );
            tyrus_graph_add_edge(
                graph,
                node_ptr(&mut c),
                node_ptr(&mut b),
                weight_ptr(&mut w),
            );
            assert_eq!(tyrus_graph_edge_count(graph), 4);

            assert!(tyrus_graph_remove_node(graph, node_ptr(&mut b)));
            assert!(!tyrus_graph_remove_node(graph, node_ptr(&mut b)));
            assert_eq!(tyrus_graph_node_count(graph), 2);
            assert_eq!(
                tyrus_graph_edge_count(graph),
                0,
                "all incident edges removed"
            );
            assert!(!tyrus_graph_has_edge(
                graph,
                node_ptr(&mut a),
                node_ptr(&mut c)
            ));

            tyrus_graph_add_edge(
                graph,
                node_ptr(&mut c),
                node_ptr(&mut a),
                weight_ptr(&mut w),
            );
            assert!(tyrus_graph_has_edge(
                graph,
                node_ptr(&mut c),
                node_ptr(&mut a)
            ));
            assert_eq!(tyrus_graph_degree(graph, node_ptr(&mut c)), 1);

            tyrus_graph_free(graph);
        }
    }

    #[test]
    fn string_nodes_compare_by_content() {
        let graph = unsafe { tyrus_graph_new(8, 8, GRAPH_TAG_STRING, GRAPH_TAG_SIGNED) };
        let sp_a = TyrusString::literal("SP");
        let sp_b = TyrusString::literal("SP");
        let rio = TyrusString::literal("Rio");
        let mut w = 429i64;
        let sp_a_slot = &sp_a;
        let sp_b_slot = &sp_b;
        let rio_slot = &rio;
        unsafe {
            tyrus_graph_add_edge(
                graph,
                sp_a_slot as *const TyrusString as *const u8,
                rio_slot as *const TyrusString as *const u8,
                weight_ptr(&mut w),
            );
            assert!(tyrus_graph_has_node(
                graph,
                sp_b_slot as *const TyrusString as *const u8
            ));
            assert_eq!(tyrus_graph_node_count(graph), 2);
            assert!(tyrus_graph_has_edge(
                graph,
                sp_b_slot as *const TyrusString as *const u8,
                rio_slot as *const TyrusString as *const u8,
            ));

            let dot = tyrus_graph_to_dot(graph);
            assert!(
                dot.as_str().contains("\"SP\" -> \"Rio\" [label=\"429\"];"),
                "{}",
                dot.as_str()
            );
            dot.release();

            let text = tyrus_graph_to_string(graph);
            assert_eq!(text.as_str(), "Graph { \"SP\" -> \"Rio\" [label=\"429\"] }");
            text.release();

            tyrus_graph_free(graph);
        }
    }

    #[test]
    fn reverse_mirrors_edge_directions() {
        let graph = unsafe { tyrus_graph_new(4, 4, GRAPH_TAG_SIGNED, GRAPH_TAG_SIGNED) };
        let mut a = 1i32;
        let mut b = 2i32;
        let mut w = 7i32;
        unsafe {
            tyrus_graph_add_edge(
                graph,
                node_ptr(&mut a),
                node_ptr(&mut b),
                weight_ptr(&mut w),
            );
            let reversed = tyrus_graph_reverse(graph);
            assert!(tyrus_graph_has_edge(
                reversed,
                node_ptr(&mut b),
                node_ptr(&mut a)
            ));
            assert!(!tyrus_graph_has_edge(
                reversed,
                node_ptr(&mut a),
                node_ptr(&mut b)
            ));
            assert_eq!(tyrus_graph_node_count(reversed), 2);
            tyrus_graph_free(reversed);
            tyrus_graph_free(graph);
        }
    }

    #[test]
    fn neighbors_and_edges_collect_in_insertion_order() {
        let graph = unsafe { tyrus_graph_new(4, 4, GRAPH_TAG_SIGNED, GRAPH_TAG_SIGNED) };
        let mut a = 1i32;
        let mut b = 2i32;
        let mut c = 3i32;
        let mut w1 = 10i32;
        let mut w2 = 20i32;
        unsafe {
            tyrus_graph_add_edge(
                graph,
                node_ptr(&mut a),
                node_ptr(&mut b),
                weight_ptr(&mut w1),
            );
            tyrus_graph_add_edge(
                graph,
                node_ptr(&mut a),
                node_ptr(&mut c),
                weight_ptr(&mut w2),
            );

            let mut output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            tyrus_graph_neighbors(graph, node_ptr(&mut a), &mut output);
            assert_eq!(output.length, 2);
            let targets = std::slice::from_raw_parts(output.data.cast::<i32>(), output.length);
            assert_eq!(targets, &[2, 3]);
            free(output.data.cast());

            let mut sources = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            let mut targets_out = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            let mut weights = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            tyrus_graph_edges_sources(graph, &mut sources);
            tyrus_graph_edges_targets(graph, &mut targets_out);
            tyrus_graph_edges_weights(graph, &mut weights);
            assert_eq!(sources.length, 2);
            assert_eq!(targets_out.length, 2);
            assert_eq!(weights.length, 2);
            let src = std::slice::from_raw_parts(sources.data.cast::<i32>(), sources.length);
            let dst =
                std::slice::from_raw_parts(targets_out.data.cast::<i32>(), targets_out.length);
            let wts = std::slice::from_raw_parts(weights.data.cast::<i32>(), weights.length);
            assert_eq!(src, &[1, 1]);
            assert_eq!(dst, &[2, 3]);
            assert_eq!(wts, &[10, 20]);
            free(sources.data.cast());
            free(targets_out.data.cast());
            free(weights.data.cast());

            tyrus_graph_free(graph);
        }
    }

    #[test]
    fn float_nodes_compare_by_bytes_and_render_like_console_log() {
        let graph = unsafe { tyrus_graph_new(8, 4, GRAPH_TAG_FLOAT, GRAPH_TAG_FLOAT) };
        let x = 1.5f64;
        let y = 2.0f64;
        let w = 0.5f32;
        unsafe {
            tyrus_graph_add_edge(
                graph,
                x.to_bits().to_ne_bytes().as_ptr().cast(),
                y.to_bits().to_ne_bytes().as_ptr().cast(),
                w.to_bits().to_ne_bytes().as_ptr().cast(),
            );
            let text = tyrus_graph_to_string(graph);
            assert_eq!(
                text.as_str(),
                "Graph { \"1.5\" -> \"2.0\" [label=\"0.5\"] }"
            );
            text.release();
            tyrus_graph_free(graph);
        }
    }
}
