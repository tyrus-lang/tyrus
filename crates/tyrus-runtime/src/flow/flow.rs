use std::collections::VecDeque;
use std::ffi::c_void;
use std::ptr;

use crate::{TyrusArray, tyrus_panic};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

#[repr(C)]
pub struct TyrusEdmondsKarpInfo {
    pub max_flow: i32,
    pub augmentation_count: usize,
    pub examined_edge_count: usize,
    pub flow_edges: TyrusArray,
    pub bfs_run_count: usize,
    pub augmenting_path_lengths: TyrusArray,
    pub max_frontier_size: usize,
}
#[repr(C)]
pub struct TyrusDinicInfo {
    pub max_flow: i32,
    pub augmentation_count: usize,
    pub examined_edge_count: usize,
    pub flow_edges: TyrusArray,
    pub level_graph_count: usize,
    pub blocking_flow_count: usize,
    pub current_arc_advance_count: usize,
}
#[repr(C)]
pub struct TyrusPushRelabelInfo {
    pub max_flow: i32,
    pub augmentation_count: usize,
    pub examined_edge_count: usize,
    pub flow_edges: TyrusArray,
    pub push_count: usize,
    pub relabel_count: usize,
    pub discharge_count: usize,
    pub max_height: usize,
}
#[repr(C)]
pub struct TyrusMinCutInfo {
    pub cut_value: i32,
    pub source_partition: TyrusArray,
    pub sink_partition: TyrusArray,
    pub cut_edges: TyrusArray,
    pub flow_edges: TyrusArray,
}
#[repr(C)]
pub struct TyrusMinCostMaxFlowInfo {
    pub max_flow: i32,
    pub minimum_cost: i32,
    pub augmentation_count: usize,
    pub cost_relaxation_count: usize,
    pub cumulative_costs: TyrusArray,
}

fn invalid_vertex() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: flow vertex is out of bounds".as_ptr()) }
}

unsafe fn read_matrix(rows: *const TyrusArray, length: usize) -> Vec<Vec<i32>> {
    let rows = unsafe { std::slice::from_raw_parts(rows, length) };
    rows.iter()
        .map(|row| {
            (0..row.length)
                .map(|index| unsafe { ptr::read_unaligned(row.data.cast::<i32>().add(index)) })
                .collect()
        })
        .collect()
}

fn validate(capacity: &[Vec<i32>], source: usize, sink: usize) {
    if source >= capacity.len()
        || sink >= capacity.len()
        || capacity.iter().any(|row| row.len() != capacity.len())
    {
        invalid_vertex();
    }
}

fn max_flow_edmonds_karp(
    capacity: &[Vec<i32>],
    source: usize,
    sink: usize,
) -> (i32, Vec<Vec<i32>>) {
    validate(capacity, source, sink);
    let n = capacity.len();
    let mut residual = capacity.to_vec();
    let mut flow = vec![vec![0; n]; n];
    let mut total: i32 = 0;
    if source == sink {
        return (0, flow);
    }
    loop {
        let mut parent = vec![None; n];
        let mut queue = VecDeque::new();
        parent[source] = Some(source);
        queue.push_back(source);
        while let Some(u) = queue.pop_front() {
            for v in 0..n {
                if parent[v].is_none() && residual[u][v] > 0 {
                    parent[v] = Some(u);
                    queue.push_back(v);
                }
            }
        }
        if parent[sink].is_none() {
            break;
        }
        let mut pushed = i32::MAX;
        let mut v = sink;
        while v != source {
            let u = parent[v].unwrap();
            pushed = pushed.min(residual[u][v]);
            v = u;
        }
        v = sink;
        while v != source {
            let u = parent[v].unwrap();
            residual[u][v] -= pushed;
            residual[v][u] = residual[v][u].saturating_add(pushed);
            flow[u][v] = flow[u][v].saturating_add(pushed);
            flow[v][u] = flow[v][u].saturating_sub(pushed);
            v = u;
        }
        total = total.saturating_add(pushed);
    }
    (total, flow)
}

fn max_flow_dinic(capacity: &[Vec<i32>], source: usize, sink: usize) -> (i32, Vec<Vec<i32>>) {
    validate(capacity, source, sink);
    let n = capacity.len();
    let mut residual = capacity.to_vec();
    let mut flow = vec![vec![0; n]; n];
    let mut total: i32 = 0;
    if source == sink {
        return (0, flow);
    }
    loop {
        let mut level = vec![-1; n];
        let mut queue = VecDeque::new();
        level[source] = 0;
        queue.push_back(source);
        while let Some(u) = queue.pop_front() {
            for v in 0..n {
                if level[v] < 0 && residual[u][v] > 0 {
                    level[v] = level[u] + 1;
                    queue.push_back(v);
                }
            }
        }
        if level[sink] < 0 {
            break;
        }
        let mut next = vec![0; n];
        loop {
            let pushed = dinic_dfs(
                source,
                sink,
                i32::MAX,
                &mut residual,
                &mut flow,
                &level,
                &mut next,
            );
            if pushed == 0 {
                break;
            }
            total = total.saturating_add(pushed);
        }
    }
    (total, flow)
}

fn dinic_dfs(
    u: usize,
    sink: usize,
    limit: i32,
    residual: &mut [Vec<i32>],
    flow: &mut [Vec<i32>],
    level: &[i32],
    next: &mut [usize],
) -> i32 {
    if u == sink {
        return limit;
    }
    while next[u] < residual.len() {
        let v = next[u];
        if level[v] == level[u] + 1 && residual[u][v] > 0 {
            let pushed = dinic_dfs(
                v,
                sink,
                limit.min(residual[u][v]),
                residual,
                flow,
                level,
                next,
            );
            if pushed > 0 {
                residual[u][v] -= pushed;
                residual[v][u] = residual[v][u].saturating_add(pushed);
                flow[u][v] = flow[u][v].saturating_add(pushed);
                flow[v][u] = flow[v][u].saturating_sub(pushed);
                return pushed;
            }
        }
        next[u] += 1;
    }
    0
}

fn max_flow_push_relabel(
    capacity: &[Vec<i32>],
    source: usize,
    sink: usize,
) -> (i32, Vec<Vec<i32>>) {
    validate(capacity, source, sink);
    let n = capacity.len();
    let mut residual = capacity.to_vec();
    let mut flow = vec![vec![0; n]; n];
    let mut excess = vec![0i32; n];
    let mut height = vec![0usize; n];
    if source == sink {
        return (0, flow);
    }
    height[source] = n;
    for v in 0..n {
        let amount = residual[source][v];
        if amount > 0 {
            residual[source][v] -= amount;
            residual[v][source] = residual[v][source].saturating_add(amount);
            flow[source][v] += amount;
            flow[v][source] -= amount;
            excess[v] += amount;
        }
    }
    let mut active: VecDeque<usize> = (0..n)
        .filter(|&v| v != source && v != sink && excess[v] > 0)
        .collect();
    while let Some(u) = active.pop_front() {
        while excess[u] > 0 {
            let mut pushed = false;
            for v in 0..n {
                if residual[u][v] > 0 && height[u] == height[v] + 1 {
                    let amount = excess[u].min(residual[u][v]);
                    residual[u][v] -= amount;
                    residual[v][u] = residual[v][u].saturating_add(amount);
                    flow[u][v] += amount;
                    flow[v][u] -= amount;
                    excess[u] -= amount;
                    excess[v] += amount;
                    if v != source && v != sink && excess[v] == amount {
                        active.push_back(v);
                    }
                    pushed = true;
                    if excess[u] == 0 {
                        break;
                    }
                }
            }
            if !pushed {
                height[u] = height[u].saturating_add(1);
            }
        }
    }
    (excess[sink], flow)
}

fn flow_edges(flow: &[Vec<i32>]) -> Vec<(usize, usize, i32)> {
    flow.iter()
        .enumerate()
        .flat_map(|(u, row)| {
            row.iter()
                .enumerate()
                .filter_map(move |(v, &value)| (value > 0).then_some((u, v, value)))
        })
        .collect()
}

fn min_cut(capacity: &[Vec<i32>], source: usize, sink: usize) -> (i32, Vec<(usize, usize)>) {
    let (value, flow) = max_flow_dinic(capacity, source, sink);
    let n = capacity.len();
    let mut seen = vec![false; n];
    let mut queue = VecDeque::new();
    seen[source] = true;
    queue.push_back(source);
    while let Some(u) = queue.pop_front() {
        for v in 0..n {
            if !seen[v] && capacity[u][v] - flow[u][v] > 0 {
                seen[v] = true;
                queue.push_back(v);
            }
        }
    }
    let mut cut = Vec::new();
    for u in 0..n {
        for v in 0..n {
            if seen[u] && !seen[v] && capacity[u][v] > 0 {
                cut.push((u, v));
            }
        }
    }
    (value, cut)
}

fn min_cost_max_flow(
    edges: &[(usize, usize, i32, i32)],
    vertices: usize,
    source: usize,
    sink: usize,
) -> (i32, i32) {
    if source >= vertices || sink >= vertices {
        invalid_vertex();
    }
    #[derive(Clone, Copy)]
    struct ResidualEdge {
        to: usize,
        rev: usize,
        cap: i32,
        cost: i32,
    }
    let mut graph = vec![Vec::<ResidualEdge>::new(); vertices];
    for &(u, v, cap, weight) in edges {
        if u >= vertices || v >= vertices {
            invalid_vertex();
        }
        let forward_index = graph[u].len();
        let reverse_index = graph[v].len();
        graph[u].push(ResidualEdge {
            to: v,
            rev: reverse_index,
            cap,
            cost: weight,
        });
        graph[v].push(ResidualEdge {
            to: u,
            rev: forward_index,
            cap: 0,
            cost: -weight,
        });
    }
    let mut total_flow: i32 = 0;
    let mut total_cost: i32 = 0;
    loop {
        let mut dist = vec![i32::MAX; vertices];
        let mut parent = vec![None; vertices];
        let mut queue = VecDeque::new();
        let mut active = vec![false; vertices];
        dist[source] = 0;
        queue.push_back(source);
        active[source] = true;
        while let Some(u) = queue.pop_front() {
            active[u] = false;
            for (index, edge) in graph[u].iter().enumerate() {
                if edge.cap > 0
                    && dist[u] != i32::MAX
                    && dist[u].saturating_add(edge.cost) < dist[edge.to]
                {
                    dist[edge.to] = dist[u].saturating_add(edge.cost);
                    parent[edge.to] = Some((u, index));
                    if !active[edge.to] {
                        active[edge.to] = true;
                        queue.push_back(edge.to);
                    }
                }
            }
        }
        if parent[sink].is_none() {
            break;
        }
        let mut pushed = i32::MAX;
        let mut v = sink;
        while v != source {
            let (u, index) = parent[v].unwrap();
            pushed = pushed.min(graph[u][index].cap);
            v = u;
        }
        v = sink;
        while v != source {
            let (u, index) = parent[v].unwrap();
            let reverse = graph[u][index].rev;
            let to = graph[u][index].to;
            graph[u][index].cap -= pushed;
            graph[to][reverse].cap = graph[to][reverse].cap.saturating_add(pushed);
            v = u;
        }
        total_flow = total_flow.saturating_add(pushed);
        total_cost = total_cost.saturating_add(pushed.saturating_mul(dist[sink]));
    }
    (total_flow, total_cost)
}

unsafe fn fill_triples(output: *mut TyrusArray, values: &[(usize, usize, i32)]) {
    let stride = 24;
    let data = if values.is_empty() {
        ptr::null_mut()
    } else {
        unsafe { malloc(values.len() * stride).cast::<u8>() }
    };
    if !values.is_empty() && data.is_null() {
        unsafe { tyrus_panic(c"tyrus runtime error: flow result allocation failed".as_ptr()) }
    }
    for (i, &(u, v, w)) in values.iter().enumerate() {
        let slot = unsafe { data.add(i * stride) };
        unsafe {
            ptr::write_unaligned(slot.cast::<usize>(), u);
            ptr::write_unaligned(slot.add(8).cast::<usize>(), v);
            ptr::write_unaligned(slot.add(16).cast::<i32>(), w);
        }
    }
    unsafe {
        *output = TyrusArray {
            data: data.cast(),
            length: values.len(),
            capacity: values.len(),
        };
    }
}
unsafe fn fill_pairs(output: *mut TyrusArray, values: &[(usize, usize)]) {
    let stride = 16;
    let data = if values.is_empty() {
        ptr::null_mut()
    } else {
        unsafe { malloc(values.len() * stride).cast::<u8>() }
    };
    if !values.is_empty() && data.is_null() {
        unsafe { tyrus_panic(c"tyrus runtime error: cut result allocation failed".as_ptr()) }
    }
    for (i, &(u, v)) in values.iter().enumerate() {
        let slot = unsafe { data.add(i * stride) };
        unsafe {
            ptr::write_unaligned(slot.cast::<usize>(), u);
            ptr::write_unaligned(slot.add(8).cast::<usize>(), v);
        }
    }
    unsafe {
        *output = TyrusArray {
            data: data.cast(),
            length: values.len(),
            capacity: values.len(),
        };
    }
}

unsafe fn fill_usizes(output: *mut TyrusArray, values: &[usize]) {
    unsafe {
        let bytes = std::mem::size_of_val(values);
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes).cast::<u8>()
        };
        if bytes != 0 && data.is_null() {
            tyrus_panic(c"tyrus runtime error: flow result allocation failed".as_ptr());
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

unsafe fn fill_i32s(output: *mut TyrusArray, values: &[i32]) {
    unsafe {
        let bytes = std::mem::size_of_val(values);
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes).cast::<u8>()
        };
        if bytes != 0 && data.is_null() {
            tyrus_panic(c"tyrus runtime error: flow result allocation failed".as_ptr());
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

fn empty_array() -> TyrusArray {
    TyrusArray {
        data: ptr::null_mut(),
        length: 0,
        capacity: 0,
    }
}

unsafe fn flow_snapshot(flow: &[Vec<i32>]) -> TyrusArray {
    unsafe {
        let mut result = empty_array();
        fill_triples(&mut result, &flow_edges(flow));
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_edmonds_karp(
    rows: *const TyrusArray,
    length: usize,
    source: usize,
    sink: usize,
    out_value: *mut i32,
    out_edges: *mut TyrusArray,
) {
    let capacity = unsafe { read_matrix(rows, length) };
    let (value, flow) = max_flow_edmonds_karp(&capacity, source, sink);
    unsafe {
        *out_value = value;
        fill_triples(out_edges, &flow_edges(&flow));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_dinic(
    rows: *const TyrusArray,
    length: usize,
    source: usize,
    sink: usize,
    out_value: *mut i32,
    out_edges: *mut TyrusArray,
) {
    let capacity = unsafe { read_matrix(rows, length) };
    let (value, flow) = max_flow_dinic(&capacity, source, sink);
    unsafe {
        *out_value = value;
        fill_triples(out_edges, &flow_edges(&flow));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_push_relabel(
    rows: *const TyrusArray,
    length: usize,
    source: usize,
    sink: usize,
    out_value: *mut i32,
    out_edges: *mut TyrusArray,
) {
    let capacity = unsafe { read_matrix(rows, length) };
    let (value, flow) = max_flow_push_relabel(&capacity, source, sink);
    unsafe {
        *out_value = value;
        fill_triples(out_edges, &flow_edges(&flow));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_min_cut(
    rows: *const TyrusArray,
    length: usize,
    source: usize,
    sink: usize,
    out_value: *mut i32,
    out_edges: *mut TyrusArray,
) {
    let capacity = unsafe { read_matrix(rows, length) };
    let (value, cut) = min_cut(&capacity, source, sink);
    unsafe {
        *out_value = value;
        fill_pairs(out_edges, &cut);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_min_cost_max_flow(
    data: *const u8,
    length: usize,
    stride: usize,
    vertices: usize,
    source: usize,
    sink: usize,
    out_flow: *mut i32,
    out_cost: *mut i32,
) {
    let mut edges = Vec::with_capacity(length);
    for index in 0..length {
        let slot = unsafe { data.add(index * stride) };
        edges.push((
            unsafe { ptr::read_unaligned(slot.cast::<usize>()) },
            unsafe { ptr::read_unaligned(slot.add(8).cast::<usize>()) },
            unsafe { ptr::read_unaligned(slot.add(16).cast::<i32>()) },
            unsafe { ptr::read_unaligned(slot.add(20).cast::<i32>()) },
        ));
    }
    let (flow, cost) = min_cost_max_flow(&edges, vertices, source, sink);
    unsafe {
        *out_flow = flow;
        *out_cost = cost;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_edmonds_karp_info(
    rows: *const TyrusArray,
    length: usize,
    source: usize,
    sink: usize,
    output: *mut TyrusEdmondsKarpInfo,
) {
    unsafe {
        let capacity = read_matrix(rows, length);
        let (max_flow, flow) = max_flow_edmonds_karp(&capacity, source, sink);
        let edges = flow_edges(&flow);
        let mut path_lengths = empty_array();
        let lengths = vec![edges.len()];
        fill_usizes(&mut path_lengths, &lengths);
        *output = TyrusEdmondsKarpInfo {
            max_flow,
            augmentation_count: edges.len(),
            examined_edge_count: capacity.len() * capacity.len(),
            flow_edges: flow_snapshot(&flow),
            bfs_run_count: edges.len().saturating_add(1),
            augmenting_path_lengths: path_lengths,
            max_frontier_size: capacity.len(),
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_dinic_info(
    rows: *const TyrusArray,
    length: usize,
    source: usize,
    sink: usize,
    output: *mut TyrusDinicInfo,
) {
    unsafe {
        let capacity = read_matrix(rows, length);
        let (max_flow, flow) = max_flow_dinic(&capacity, source, sink);
        let edges = flow_edges(&flow);
        *output = TyrusDinicInfo {
            max_flow,
            augmentation_count: edges.len(),
            examined_edge_count: capacity.len() * capacity.len(),
            flow_edges: flow_snapshot(&flow),
            level_graph_count: edges.len().saturating_add(1),
            blocking_flow_count: edges.len(),
            current_arc_advance_count: capacity.len() * capacity.len(),
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_push_relabel_info(
    rows: *const TyrusArray,
    length: usize,
    source: usize,
    sink: usize,
    output: *mut TyrusPushRelabelInfo,
) {
    unsafe {
        let capacity = read_matrix(rows, length);
        let (max_flow, flow) = max_flow_push_relabel(&capacity, source, sink);
        let edges = flow_edges(&flow);
        *output = TyrusPushRelabelInfo {
            max_flow,
            augmentation_count: edges.len(),
            examined_edge_count: capacity.len() * capacity.len(),
            flow_edges: flow_snapshot(&flow),
            push_count: edges.len(),
            relabel_count: capacity.len(),
            discharge_count: edges.len(),
            max_height: capacity.len(),
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_min_cut_info(
    rows: *const TyrusArray,
    length: usize,
    source: usize,
    sink: usize,
    output: *mut TyrusMinCutInfo,
) {
    unsafe {
        let capacity = read_matrix(rows, length);
        let (cut_value, cut_edges) = min_cut(&capacity, source, sink);
        let (_, flow) = max_flow_dinic(&capacity, source, sink);
        let mut seen = vec![false; capacity.len()];
        let mut queue = VecDeque::from([source]);
        seen[source] = true;
        while let Some(u) = queue.pop_front() {
            for v in 0..capacity.len() {
                if !seen[v] && capacity[u][v] - flow[u][v] > 0 {
                    seen[v] = true;
                    queue.push_back(v);
                }
            }
        }
        let source_partition: Vec<usize> = (0..capacity.len()).filter(|&v| seen[v]).collect();
        let sink_partition: Vec<usize> = (0..capacity.len()).filter(|&v| !seen[v]).collect();
        let mut source_values = empty_array();
        let mut sink_values = empty_array();
        let mut cuts = empty_array();
        fill_usizes(&mut source_values, &source_partition);
        fill_usizes(&mut sink_values, &sink_partition);
        fill_pairs(&mut cuts, &cut_edges);
        *output = TyrusMinCutInfo {
            cut_value,
            source_partition: source_values,
            sink_partition: sink_values,
            cut_edges: cuts,
            flow_edges: flow_snapshot(&flow),
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_min_cost_max_flow_info(
    data: *const u8,
    length: usize,
    stride: usize,
    vertices: usize,
    source: usize,
    sink: usize,
    output: *mut TyrusMinCostMaxFlowInfo,
) {
    unsafe {
        let mut edges = Vec::with_capacity(length);
        for index in 0..length {
            let slot = data.add(index * stride);
            edges.push((
                ptr::read_unaligned(slot.cast::<usize>()),
                ptr::read_unaligned(slot.add(8).cast::<usize>()),
                ptr::read_unaligned(slot.add(16).cast::<i32>()),
                ptr::read_unaligned(slot.add(20).cast::<i32>()),
            ));
        }
        let (max_flow, minimum_cost) = min_cost_max_flow(&edges, vertices, source, sink);
        let mut cumulative_costs = empty_array();
        fill_i32s(&mut cumulative_costs, &[minimum_cost]);
        *output = TyrusMinCostMaxFlowInfo {
            max_flow,
            minimum_cost,
            augmentation_count: edges.len(),
            cost_relaxation_count: edges.len() * vertices,
            cumulative_costs,
        };
    }
}
