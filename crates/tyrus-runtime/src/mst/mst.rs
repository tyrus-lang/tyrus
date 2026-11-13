use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::ffi::c_void;
use std::ptr;

use crate::{TyrusArray, TyrusGraph, tyrus_panic};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Edge {
    u: usize,
    v: usize,
    w: i32,
}

#[repr(C)]
pub struct TyrusKruskalInfo {
    pub edges: TyrusArray,
    pub total_weight: i32,
    pub component_count: usize,
    pub examined_edge_count: usize,
    pub accepted_edge_count: usize,
    pub union_attempt_count: usize,
    pub successful_union_count: usize,
    pub sorted_edge_examination_count: usize,
}

#[repr(C)]
pub struct TyrusPrimInfo {
    pub edges: TyrusArray,
    pub total_weight: i32,
    pub component_count: usize,
    pub examined_edge_count: usize,
    pub accepted_edge_count: usize,
    pub heap_push_count: usize,
    pub heap_pop_count: usize,
    pub key_update_count: usize,
}

#[repr(C)]
pub struct TyrusBoruvkaInfo {
    pub edges: TyrusArray,
    pub total_weight: i32,
    pub component_count: usize,
    pub examined_edge_count: usize,
    pub accepted_edge_count: usize,
    pub phase_count: usize,
    pub component_counts_after_phase: TyrusArray,
}

#[repr(C)]
pub struct TyrusSteinerTreeInfo {
    pub edges: TyrusArray,
    pub total_weight: i32,
    pub terminals: TyrusArray,
    pub evaluated_subset_count: usize,
    pub vertex_count: usize,
    pub max_terminal_count: usize,
    pub within_terminal_limit: bool,
}

struct DisjointSet {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl DisjointSet {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    fn find(&mut self, value: usize) -> usize {
        if self.parent[value] != value {
            self.parent[value] = self.find(self.parent[value]);
        }
        self.parent[value]
    }

    fn union(&mut self, left: usize, right: usize) -> bool {
        let mut left = self.find(left);
        let mut right = self.find(right);
        if left == right {
            return false;
        }
        if self.rank[left] < self.rank[right] {
            std::mem::swap(&mut left, &mut right);
        }
        self.parent[right] = left;
        if self.rank[left] == self.rank[right] {
            self.rank[left] += 1;
        }
        true
    }

    fn component_count(&mut self) -> usize {
        let mut roots = std::collections::BTreeSet::new();
        for vertex in 0..self.parent.len() {
            roots.insert(self.find(vertex));
        }
        roots.len()
    }
}

fn invalid_vertex() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: MST vertex is out of bounds".as_ptr()) }
}

unsafe fn read_edges(data: *const u8, length: usize, stride: usize) -> Vec<Edge> {
    let mut edges = Vec::with_capacity(length);
    for index in 0..length {
        let slot = unsafe { data.add(index * stride) };
        let u = unsafe { ptr::read_unaligned(slot.cast::<usize>()) };
        let v = unsafe { ptr::read_unaligned(slot.add(8).cast::<usize>()) };
        let w = unsafe { ptr::read_unaligned(slot.add(16).cast::<i32>()) };
        edges.push(Edge { u, v, w });
    }
    edges
}

unsafe fn read_adjacency(
    rows: *const TyrusArray,
    length: usize,
    stride: usize,
) -> Vec<Vec<(usize, i32)>> {
    let rows = unsafe { std::slice::from_raw_parts(rows, length) };
    rows.iter()
        .map(|row| {
            (0..row.length)
                .map(|index| {
                    let slot = unsafe { row.data.cast::<u8>().add(index * stride) };
                    let vertex = unsafe { ptr::read_unaligned(slot.cast::<usize>()) };
                    let weight = unsafe { ptr::read_unaligned(slot.add(8).cast::<i32>()) };
                    (vertex, weight)
                })
                .collect()
        })
        .collect()
}

fn graph_edges(graph: &TyrusGraph) -> Vec<Edge> {
    graph
        .adjacency()
        .iter()
        .enumerate()
        .flat_map(|(u, neighbors)| {
            neighbors.iter().map(move |edge| Edge {
                u,
                v: edge.to,
                w: graph.weight_as_i32(&edge.weight),
            })
        })
        .collect()
}

fn graph_adjacency(graph: &TyrusGraph) -> Vec<Vec<(usize, i32)>> {
    graph
        .adjacency()
        .iter()
        .map(|neighbors| {
            neighbors
                .iter()
                .map(|edge| (edge.to, graph.weight_as_i32(&edge.weight)))
                .collect()
        })
        .collect()
}

fn kruskal_impl(mut edges: Vec<Edge>, vertices: usize) -> Vec<Edge> {
    let mut set = DisjointSet::new(vertices);
    edges.sort_by_key(|edge| edge.w);
    edges
        .into_iter()
        .filter(|edge| {
            if edge.u >= vertices || edge.v >= vertices {
                invalid_vertex();
            }
            set.union(edge.u, edge.v)
        })
        .take(vertices.saturating_sub(1))
        .collect()
}

fn kruskal_info_impl(
    mut edges: Vec<Edge>,
    vertices: usize,
) -> (Vec<Edge>, usize, usize, usize, usize) {
    let mut set = DisjointSet::new(vertices);
    edges.sort_by_key(|edge| edge.w);
    let mut result = Vec::new();
    let mut attempts = 0;
    for edge in edges {
        if edge.u >= vertices || edge.v >= vertices {
            invalid_vertex();
        }
        attempts += 1;
        if set.union(edge.u, edge.v) {
            result.push(edge);
            if result.len() == vertices.saturating_sub(1) {
                break;
            }
        }
    }
    let components = set.component_count();
    (result, components, attempts, attempts, attempts)
}

fn prim_impl(adjacency: &[Vec<(usize, i32)>], origin: usize) -> Vec<Edge> {
    if origin >= adjacency.len() {
        invalid_vertex();
    }
    let mut visited = vec![false; adjacency.len()];
    let mut heap = BinaryHeap::new();
    let mut result = Vec::new();
    visited[origin] = true;
    for &(vertex, weight) in &adjacency[origin] {
        heap.push(Reverse((weight, origin, vertex)));
    }
    while let Some(Reverse((weight, from, to))) = heap.pop() {
        if to >= adjacency.len() {
            invalid_vertex();
        }
        if visited[to] {
            continue;
        }
        visited[to] = true;
        result.push(Edge {
            u: from,
            v: to,
            w: weight,
        });
        for &(next, next_weight) in &adjacency[to] {
            if next >= adjacency.len() {
                invalid_vertex();
            }
            if !visited[next] {
                heap.push(Reverse((next_weight, to, next)));
            }
        }
    }
    result
}

fn prim_info_impl(
    adjacency: &[Vec<(usize, i32)>],
    origin: usize,
) -> (Vec<Edge>, usize, usize, usize, usize) {
    if origin >= adjacency.len() {
        invalid_vertex();
    }
    let mut visited = vec![false; adjacency.len()];
    let mut heap = BinaryHeap::new();
    let mut result = Vec::new();
    let (mut pushes, mut pops, mut examined) = (0, 0, 0);
    visited[origin] = true;
    for &(vertex, weight) in &adjacency[origin] {
        heap.push(Reverse((weight, origin, vertex)));
        pushes += 1;
    }
    while let Some(Reverse((weight, from, to))) = heap.pop() {
        pops += 1;
        if to >= adjacency.len() {
            invalid_vertex();
        }
        if visited[to] {
            continue;
        }
        visited[to] = true;
        result.push(Edge {
            u: from,
            v: to,
            w: weight,
        });
        for &(next, next_weight) in &adjacency[to] {
            examined += 1;
            if next >= adjacency.len() {
                invalid_vertex();
            }
            if !visited[next] {
                heap.push(Reverse((next_weight, to, next)));
                pushes += 1;
            }
        }
    }
    let components = adjacency.len().saturating_sub(result.len());
    (result, components, examined, pushes, pops)
}

fn boruvka_impl(edges: &[Edge], vertices: usize) -> Vec<Edge> {
    let mut set = DisjointSet::new(vertices);
    let mut result = Vec::new();
    while result.len() < vertices.saturating_sub(1) {
        let mut cheapest = vec![None; vertices];
        for &edge in edges {
            if edge.u >= vertices || edge.v >= vertices {
                invalid_vertex();
            }
            let left = set.find(edge.u);
            let right = set.find(edge.v);
            if left == right {
                continue;
            }
            if cheapest[left].is_none_or(|current: Edge| edge.w < current.w) {
                cheapest[left] = Some(edge);
            }
            if cheapest[right].is_none_or(|current: Edge| edge.w < current.w) {
                cheapest[right] = Some(edge);
            }
        }
        let mut merged = false;
        for edge in cheapest.into_iter().flatten() {
            if set.union(edge.u, edge.v) {
                result.push(edge);
                merged = true;
            }
        }
        if !merged {
            break;
        }
    }
    result
}

fn boruvka_info_impl(
    edges: &[Edge],
    vertices: usize,
) -> (Vec<Edge>, usize, usize, usize, Vec<usize>) {
    let mut set = DisjointSet::new(vertices);
    let mut result = Vec::new();
    let (mut examined, mut phases, mut components_after_phase) = (0, 0, Vec::new());
    while result.len() < vertices.saturating_sub(1) {
        let mut cheapest = vec![None; vertices];
        for &edge in edges {
            if edge.u >= vertices || edge.v >= vertices {
                invalid_vertex();
            }
            examined += 1;
            let left = set.find(edge.u);
            let right = set.find(edge.v);
            if left == right {
                continue;
            }
            if cheapest[left].is_none_or(|current: Edge| edge.w < current.w) {
                cheapest[left] = Some(edge);
            }
            if cheapest[right].is_none_or(|current: Edge| edge.w < current.w) {
                cheapest[right] = Some(edge);
            }
        }
        let mut merged = false;
        for edge in cheapest.into_iter().flatten() {
            if set.union(edge.u, edge.v) {
                result.push(edge);
                merged = true;
            }
        }
        if !merged {
            break;
        }
        phases += 1;
        components_after_phase.push(set.component_count());
    }
    let components = set.component_count();
    (result, components, examined, phases, components_after_phase)
}

#[derive(Clone, Copy)]
enum SteinerChoice {
    None,
    Base,
    Merge(usize),
    Relax(usize, usize),
}

#[allow(clippy::needless_range_loop)]
fn steiner_impl(edges: &[Edge], vertices: usize, terminals: &[usize]) -> Vec<Edge> {
    if terminals.is_empty() {
        return Vec::new();
    }
    if terminals.iter().any(|&terminal| terminal >= vertices) {
        invalid_vertex();
    }
    if terminals.len() >= usize::BITS as usize || terminals.len() > 20 {
        return Vec::new();
    }
    let mut adjacency = vec![Vec::<(usize, i32, usize)>::new(); vertices];
    for (edge_id, &edge) in edges.iter().enumerate() {
        if edge.u >= vertices || edge.v >= vertices {
            invalid_vertex();
        }
        adjacency[edge.u].push((edge.v, edge.w, edge_id));
        adjacency[edge.v].push((edge.u, edge.w, edge_id));
    }
    let masks = 1usize << terminals.len();
    let mut dp = vec![vec![i32::MAX; vertices]; masks];
    let mut choices = vec![vec![SteinerChoice::None; vertices]; masks];
    for (index, &terminal) in terminals.iter().enumerate() {
        dp[1usize << index][terminal] = 0;
        choices[1usize << index][terminal] = SteinerChoice::Base;
    }
    for mask in 1..masks {
        if mask.count_ones() > 1 {
            let mut sub = (mask - 1) & mask;
            while sub > 0 {
                let other = mask ^ sub;
                if other > 0 && sub < other {
                    for vertex in 0..vertices {
                        let cost = dp[sub][vertex].saturating_add(dp[other][vertex]);
                        if cost < dp[mask][vertex] {
                            dp[mask][vertex] = cost;
                            choices[mask][vertex] = SteinerChoice::Merge(sub);
                        }
                    }
                }
                sub = (sub - 1) & mask;
            }
        }
        let mut heap = BinaryHeap::new();
        for vertex in 0..vertices {
            if dp[mask][vertex] != i32::MAX {
                heap.push(Reverse((dp[mask][vertex], vertex)));
            }
        }
        while let Some(Reverse((cost, vertex))) = heap.pop() {
            if cost != dp[mask][vertex] {
                continue;
            }
            for &(next, weight, edge_id) in &adjacency[vertex] {
                let next_cost = cost.saturating_add(weight);
                if next_cost < dp[mask][next] {
                    dp[mask][next] = next_cost;
                    choices[mask][next] = SteinerChoice::Relax(vertex, edge_id);
                    heap.push(Reverse((next_cost, next)));
                }
            }
        }
    }
    let full = masks - 1;
    let Some((root, _)) = dp[full]
        .iter()
        .enumerate()
        .filter(|(_, cost)| **cost != i32::MAX)
        .min_by_key(|(_, cost)| **cost)
    else {
        return Vec::new();
    };
    fn reconstruct(
        mask: usize,
        vertex: usize,
        choices: &[Vec<SteinerChoice>],
        used: &mut Vec<usize>,
    ) {
        match choices[mask][vertex] {
            SteinerChoice::None | SteinerChoice::Base => {}
            SteinerChoice::Merge(sub) => {
                reconstruct(sub, vertex, choices, used);
                reconstruct(mask ^ sub, vertex, choices, used);
            }
            SteinerChoice::Relax(previous, edge_id) => {
                reconstruct(mask, previous, choices, used);
                used.push(edge_id);
            }
        }
    }
    let mut used = Vec::new();
    reconstruct(full, root, &choices, &mut used);
    used.sort_unstable();
    used.dedup();
    used.into_iter().map(|edge_id| edges[edge_id]).collect()
}

unsafe fn fill_edges(output: *mut TyrusArray, edges: &[Edge]) {
    let stride = 24usize;
    let bytes = edges.len().checked_mul(stride).unwrap_or_else(|| unsafe {
        tyrus_panic(c"tyrus runtime error: MST result is too large".as_ptr())
    });
    let data = if bytes == 0 {
        ptr::null_mut()
    } else {
        unsafe { malloc(bytes).cast::<u8>() }
    };
    if bytes != 0 && data.is_null() {
        unsafe { tyrus_panic(c"tyrus runtime error: MST result allocation failed".as_ptr()) }
    }
    for (index, edge) in edges.iter().enumerate() {
        let slot = unsafe { data.add(index * stride) };
        unsafe {
            ptr::write_unaligned(slot.cast::<usize>(), edge.u);
            ptr::write_unaligned(slot.add(8).cast::<usize>(), edge.v);
            ptr::write_unaligned(slot.add(16).cast::<i32>(), edge.w);
        }
    }
    unsafe {
        *output = TyrusArray {
            data: data.cast(),
            length: edges.len(),
            capacity: edges.len(),
        };
    }
}

unsafe fn fill_usize_array(output: *mut TyrusArray, values: &[usize]) {
    unsafe {
        let bytes = values
            .len()
            .checked_mul(std::mem::size_of::<usize>())
            .unwrap_or_else(|| {
                tyrus_panic(c"tyrus runtime error: MST result is too large".as_ptr())
            });
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes).cast::<u8>()
        };
        if bytes != 0 && data.is_null() {
            tyrus_panic(c"tyrus runtime error: MST result allocation failed".as_ptr());
        }
        if bytes != 0 {
            ptr::copy_nonoverlapping(values.as_ptr().cast::<u8>(), data, bytes);
        }
        *output = TyrusArray {
            data,
            length: values.len(),
            capacity: values.len(),
        };
    }
}

fn total_weight(edges: &[Edge]) -> i32 {
    edges
        .iter()
        .fold(0i32, |total, edge| total.saturating_add(edge.w))
}

unsafe fn fill_kruskal_info(
    output: *mut TyrusKruskalInfo,
    result: (Vec<Edge>, usize, usize, usize, usize),
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let mut edges = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_edges(&mut edges, &result.0);
        *output = TyrusKruskalInfo {
            edges,
            total_weight: total_weight(&result.0),
            component_count: result.1,
            examined_edge_count: result.2,
            accepted_edge_count: result.0.len(),
            union_attempt_count: result.3,
            successful_union_count: result.0.len(),
            sorted_edge_examination_count: result.4,
        };
    }
}

unsafe fn fill_prim_info(
    output: *mut TyrusPrimInfo,
    result: (Vec<Edge>, usize, usize, usize, usize),
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let mut edges = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_edges(&mut edges, &result.0);
        *output = TyrusPrimInfo {
            edges,
            total_weight: total_weight(&result.0),
            component_count: result.1,
            examined_edge_count: result.2,
            accepted_edge_count: result.0.len(),
            heap_push_count: result.3,
            heap_pop_count: result.4,
            key_update_count: result.3,
        };
    }
}

unsafe fn fill_boruvka_info(
    output: *mut TyrusBoruvkaInfo,
    result: (Vec<Edge>, usize, usize, usize, Vec<usize>),
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let mut edges = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut components_after_phase = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_edges(&mut edges, &result.0);
        fill_usize_array(&mut components_after_phase, &result.4);
        *output = TyrusBoruvkaInfo {
            edges,
            total_weight: total_weight(&result.0),
            component_count: result.1,
            examined_edge_count: result.2,
            accepted_edge_count: result.0.len(),
            phase_count: result.3,
            component_counts_after_phase: components_after_phase,
        };
    }
}

unsafe fn fill_steiner_tree_info(
    output: *mut TyrusSteinerTreeInfo,
    edges: Vec<Edge>,
    vertices: usize,
    terminals: &[usize],
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let within_terminal_limit = terminals.len() <= 20;
        let mut tree = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut terminal_values = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_edges(&mut tree, &edges);
        fill_usize_array(&mut terminal_values, terminals);
        *output = TyrusSteinerTreeInfo {
            edges: tree,
            total_weight: total_weight(&edges),
            terminals: terminal_values,
            evaluated_subset_count: if within_terminal_limit {
                (1usize << terminals.len()).saturating_sub(1)
            } else {
                0
            },
            vertex_count: vertices,
            max_terminal_count: 20,
            within_terminal_limit,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_kruskal(
    data: *const u8,
    length: usize,
    stride: usize,
    vertices: usize,
    output: *mut TyrusArray,
) {
    let edges = unsafe { read_edges(data, length, stride) };
    let result = kruskal_impl(edges, vertices);
    unsafe { fill_edges(output, &result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_prim(
    rows: *const TyrusArray,
    length: usize,
    stride: usize,
    origin: usize,
    output: *mut TyrusArray,
) {
    let adjacency = unsafe { read_adjacency(rows, length, stride) };
    let result = prim_impl(&adjacency, origin);
    unsafe { fill_edges(output, &result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_boruvka(
    data: *const u8,
    length: usize,
    stride: usize,
    vertices: usize,
    output: *mut TyrusArray,
) {
    let edges = unsafe { read_edges(data, length, stride) };
    let result = boruvka_impl(&edges, vertices);
    unsafe { fill_edges(output, &result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_steiner_tree(
    data: *const u8,
    length: usize,
    stride: usize,
    vertices: usize,
    terminals: *const usize,
    terminal_count: usize,
    output: *mut TyrusArray,
) {
    let edges = unsafe { read_edges(data, length, stride) };
    let terminals = unsafe { std::slice::from_raw_parts(terminals, terminal_count) };
    let result = steiner_impl(&edges, vertices, terminals);
    unsafe { fill_edges(output, &result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_kruskal_info(
    data: *const u8,
    length: usize,
    stride: usize,
    vertices: usize,
    output: *mut TyrusKruskalInfo,
) {
    unsafe {
        let edges = read_edges(data, length, stride);
        fill_kruskal_info(output, kruskal_info_impl(edges, vertices));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_prim_info(
    rows: *const TyrusArray,
    length: usize,
    stride: usize,
    origin: usize,
    output: *mut TyrusPrimInfo,
) {
    unsafe {
        let adjacency = read_adjacency(rows, length, stride);
        fill_prim_info(output, prim_info_impl(&adjacency, origin));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_boruvka_info(
    data: *const u8,
    length: usize,
    stride: usize,
    vertices: usize,
    output: *mut TyrusBoruvkaInfo,
) {
    unsafe {
        let edges = read_edges(data, length, stride);
        fill_boruvka_info(output, boruvka_info_impl(&edges, vertices));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_steiner_tree_info(
    data: *const u8,
    length: usize,
    stride: usize,
    vertices: usize,
    terminals: *const usize,
    terminal_count: usize,
    output: *mut TyrusSteinerTreeInfo,
) {
    unsafe {
        let edges = read_edges(data, length, stride);
        let terminals = std::slice::from_raw_parts(terminals, terminal_count);
        let result = steiner_impl(&edges, vertices, terminals);
        fill_steiner_tree_info(output, result, vertices, terminals);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_kruskal_graph(
    graph: *const TyrusGraph,
    vertices: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let graph = &*graph;
        fill_edges(output, &kruskal_impl(graph_edges(graph), vertices));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_prim_graph(
    graph: *const TyrusGraph,
    origin: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let adjacency = graph_adjacency(&*graph);
        fill_edges(output, &prim_impl(&adjacency, origin));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_boruvka_graph(
    graph: *const TyrusGraph,
    vertices: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let graph = &*graph;
        fill_edges(output, &boruvka_impl(&graph_edges(graph), vertices));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_steiner_tree_graph(
    graph: *const TyrusGraph,
    vertices: usize,
    terminals: *const usize,
    terminal_count: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let graph = &*graph;
        let terminals = std::slice::from_raw_parts(terminals, terminal_count);
        fill_edges(
            output,
            &steiner_impl(&graph_edges(graph), vertices, terminals),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_kruskal_info_graph(
    graph: *const TyrusGraph,
    vertices: usize,
    output: *mut TyrusKruskalInfo,
) {
    unsafe {
        fill_kruskal_info(output, kruskal_info_impl(graph_edges(&*graph), vertices));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_prim_info_graph(
    graph: *const TyrusGraph,
    origin: usize,
    output: *mut TyrusPrimInfo,
) {
    unsafe {
        let adjacency = graph_adjacency(&*graph);
        fill_prim_info(output, prim_info_impl(&adjacency, origin));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_boruvka_info_graph(
    graph: *const TyrusGraph,
    vertices: usize,
    output: *mut TyrusBoruvkaInfo,
) {
    unsafe {
        fill_boruvka_info(output, boruvka_info_impl(&graph_edges(&*graph), vertices));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_steiner_tree_info_graph(
    graph: *const TyrusGraph,
    vertices: usize,
    terminals: *const usize,
    terminal_count: usize,
    output: *mut TyrusSteinerTreeInfo,
) {
    unsafe {
        let terminals = std::slice::from_raw_parts(terminals, terminal_count);
        let result = steiner_impl(&graph_edges(&*graph), vertices, terminals);
        fill_steiner_tree_info(output, result, vertices, terminals);
    }
}
