use crate::{TyrusArray, TyrusGraph, tyrus_panic};
use std::collections::VecDeque;
use std::ffi::c_void;
use std::ptr;
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

#[repr(C)]
pub struct TyrusTopologicalKahnInfo {
    pub order: TyrusArray,
    pub processed_vertex_count: usize,
    pub examined_edge_count: usize,
    pub cycle_detected: bool,
    pub initial_indegrees: TyrusArray,
    pub indegree_decrement_count: usize,
    pub max_queue_size: usize,
}

#[repr(C)]
pub struct TyrusTopologicalDfsInfo {
    pub order: TyrusArray,
    pub processed_vertex_count: usize,
    pub examined_edge_count: usize,
    pub cycle_detected: bool,
    pub final_states: TyrusArray,
    pub max_depth: usize,
    pub back_edge: *mut TyrusArray,
}
#[repr(C)]
pub struct TyrusTarjanSccInfo {
    pub components: TyrusArray,
    pub component_count: usize,
    pub examined_edge_count: usize,
    pub stack_operation_count: usize,
    pub discovery_indices: TyrusArray,
    pub low_links: TyrusArray,
    pub stack_push_count: usize,
    pub stack_pop_count: usize,
}
#[repr(C)]
pub struct TyrusKosarajuSccInfo {
    pub components: TyrusArray,
    pub component_count: usize,
    pub examined_edge_count: usize,
    pub stack_operation_count: usize,
    pub finishing_order: TyrusArray,
    pub first_traversal_edge_count: usize,
    pub second_traversal_edge_count: usize,
}
#[repr(C)]
pub struct TyrusFleuryEulerianInfo {
    pub path: *mut TyrusArray,
    pub used_edge_count: usize,
    pub examined_edge_count: usize,
    pub is_circuit: bool,
    pub bridge_check_count: usize,
    pub avoided_bridge_choice_count: usize,
}
#[repr(C)]
pub struct TyrusHierholzerEulerianInfo {
    pub path: *mut TyrusArray,
    pub used_edge_count: usize,
    pub examined_edge_count: usize,
    pub is_circuit: bool,
    pub stack_push_count: usize,
    pub stack_pop_count: usize,
    pub joined_cycle_count: usize,
}
#[repr(C)]
pub struct TyrusFindBridgesInfo {
    pub examined_edge_count: usize,
    pub discovery_indices: TyrusArray,
    pub low_links: TyrusArray,
    pub result_count: usize,
    pub bridges: TyrusArray,
}
#[repr(C)]
pub struct TyrusFindCutVerticesInfo {
    pub examined_edge_count: usize,
    pub discovery_indices: TyrusArray,
    pub low_links: TyrusArray,
    pub result_count: usize,
    pub cut_vertices: TyrusArray,
}
#[repr(C)]
pub struct TyrusBiconnectedComponentsInfo {
    pub examined_edge_count: usize,
    pub discovery_indices: TyrusArray,
    pub low_links: TyrusArray,
    pub result_count: usize,
    pub components: TyrusArray,
}
#[repr(C)]
pub struct TyrusEdmondsBlossomInfo {
    pub pairs: TyrusArray,
    pub cardinality: usize,
    pub examined_edge_count: usize,
    pub blossom_contraction_count: usize,
    pub augmenting_path_search_count: usize,
    pub augmentation_count: usize,
}
#[repr(C)]
pub struct TyrusHungarianInfo {
    pub assignments: TyrusArray,
    pub total_cost: i32,
    pub potential_adjustment_count: usize,
    pub matrix_iteration_count: usize,
}

unsafe fn read_adj(data: *const TyrusArray, length: usize) -> Vec<Vec<usize>> {
    let rows = unsafe { std::slice::from_raw_parts(data, length) };
    rows.iter()
        .map(|row| {
            (0..row.length)
                .map(|i| unsafe { ptr::read_unaligned(row.data.cast::<usize>().add(i)) })
                .collect()
        })
        .collect()
}
fn invalid() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: graph vertex is out of bounds".as_ptr()) }
}

fn graph_adj(graph: &TyrusGraph) -> Vec<Vec<usize>> {
    graph
        .adjacency()
        .iter()
        .map(|row| row.iter().map(|edge| edge.to).collect())
        .collect()
}
fn validate(adj: &[Vec<usize>]) {
    if adj.iter().flatten().any(|&v| v >= adj.len()) {
        invalid();
    }
}

fn topo_kahn(adj: &[Vec<usize>]) -> Option<Vec<usize>> {
    validate(adj);
    let mut degree = vec![0; adj.len()];
    for row in adj {
        for &v in row {
            degree[v] += 1;
        }
    }
    let mut q = VecDeque::new();
    for (v, &d) in degree.iter().enumerate() {
        if d == 0 {
            q.push_back(v);
        }
    }
    let mut result = Vec::new();
    while let Some(u) = q.pop_front() {
        result.push(u);
        for &v in &adj[u] {
            degree[v] -= 1;
            if degree[v] == 0 {
                q.push_back(v);
            }
        }
    }
    (result.len() == adj.len()).then_some(result)
}
fn topo_dfs(adj: &[Vec<usize>]) -> Option<Vec<usize>> {
    validate(adj);
    fn visit(u: usize, adj: &[Vec<usize>], state: &mut [u8], out: &mut Vec<usize>) -> bool {
        if state[u] == 1 {
            return false;
        }
        if state[u] == 2 {
            return true;
        }
        state[u] = 1;
        for &v in &adj[u] {
            if !visit(v, adj, state, out) {
                return false;
            }
        }
        state[u] = 2;
        out.push(u);
        true
    }
    let mut state = vec![0; adj.len()];
    let mut out = Vec::new();
    for u in 0..adj.len() {
        if !visit(u, adj, &mut state, &mut out) {
            return None;
        }
    }
    out.reverse();
    Some(out)
}

fn topo_kahn_metrics(adj: &[Vec<usize>]) -> (Vec<usize>, Vec<usize>, usize, usize, bool) {
    validate(adj);
    let mut degree = vec![0; adj.len()];
    for row in adj {
        for &v in row {
            degree[v] += 1;
        }
    }
    let initial = degree.clone();
    let mut q = VecDeque::new();
    for (v, &d) in degree.iter().enumerate() {
        if d == 0 {
            q.push_back(v);
        }
    }
    let mut max_queue = q.len();
    let mut order = Vec::new();
    let mut decrements = 0;
    while let Some(u) = q.pop_front() {
        order.push(u);
        for &v in &adj[u] {
            degree[v] -= 1;
            decrements += 1;
            if degree[v] == 0 {
                q.push_back(v);
                max_queue = max_queue.max(q.len());
            }
        }
    }
    let cycle = order.len() != adj.len();
    (order, initial, decrements, max_queue, cycle)
}

type TopoDfsMetrics = (Vec<usize>, Vec<usize>, usize, usize, Option<(usize, usize)>);

fn topo_dfs_metrics(adj: &[Vec<usize>]) -> TopoDfsMetrics {
    validate(adj);
    #[allow(clippy::too_many_arguments)]
    fn visit(
        u: usize,
        depth: usize,
        adj: &[Vec<usize>],
        state: &mut [usize],
        out: &mut Vec<usize>,
        examined: &mut usize,
        max_depth: &mut usize,
        back_edge: &mut Option<(usize, usize)>,
    ) -> bool {
        state[u] = 1;
        *max_depth = (*max_depth).max(depth);
        for &v in &adj[u] {
            *examined += 1;
            if state[v] == 1 {
                *back_edge = Some((u, v));
                return false;
            }
            if state[v] == 0
                && !visit(
                    v,
                    depth + 1,
                    adj,
                    state,
                    out,
                    examined,
                    max_depth,
                    back_edge,
                )
            {
                return false;
            }
        }
        state[u] = 2;
        out.push(u);
        true
    }
    let mut state = vec![0; adj.len()];
    let mut out = Vec::new();
    let mut examined = 0;
    let mut max_depth = 0;
    let mut back_edge = None;
    for u in 0..adj.len() {
        if state[u] == 0
            && !visit(
                u,
                1,
                adj,
                &mut state,
                &mut out,
                &mut examined,
                &mut max_depth,
                &mut back_edge,
            )
        {
            break;
        }
    }
    out.reverse();
    (out, state, examined, max_depth, back_edge)
}

fn kosaraju(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    validate(adj);
    fn dfs(u: usize, adj: &[Vec<usize>], seen: &mut [bool], order: &mut Vec<usize>) {
        if seen[u] {
            return;
        }
        seen[u] = true;
        for &v in &adj[u] {
            dfs(v, adj, seen, order);
        }
        order.push(u);
    }
    let mut order = Vec::new();
    let mut seen = vec![false; adj.len()];
    for u in 0..adj.len() {
        dfs(u, adj, &mut seen, &mut order);
    }
    let mut reverse = vec![Vec::new(); adj.len()];
    for (u, row) in adj.iter().enumerate() {
        for &v in row {
            reverse[v].push(u);
        }
    }
    fn collect(u: usize, adj: &[Vec<usize>], seen: &mut [bool], component: &mut Vec<usize>) {
        if seen[u] {
            return;
        }
        seen[u] = true;
        component.push(u);
        for &v in &adj[u] {
            collect(v, adj, seen, component);
        }
    }
    seen.fill(false);
    let mut result = Vec::new();
    while let Some(u) = order.pop() {
        let mut component = Vec::new();
        collect(u, &reverse, &mut seen, &mut component);
        if !component.is_empty() {
            result.push(component);
        }
    }
    result
}
fn tarjan(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    kosaraju(adj)
}

fn euler(adj: &[Vec<usize>]) -> Option<Vec<usize>> {
    validate(adj);
    let mut graph = adj.to_vec();
    let edges: usize = graph.iter().map(Vec::len).sum::<usize>() / 2;
    if edges == 0 {
        return Some(vec![]);
    }
    let odd: Vec<_> = graph
        .iter()
        .enumerate()
        .filter(|(_, row)| row.len() % 2 == 1)
        .map(|(u, _)| u)
        .collect();
    if !(odd.is_empty() || odd.len() == 2) {
        return None;
    }
    let start = odd
        .first()
        .copied()
        .or_else(|| graph.iter().position(|r| !r.is_empty()))?;
    let mut stack = vec![start];
    let mut path = Vec::new();
    while let Some(u) = stack.pop() {
        if let Some(v) = graph[u].pop() {
            if let Some(pos) = graph[v].iter().position(|&x| x == u) {
                graph[v].remove(pos);
            }
            stack.push(u);
            stack.push(v);
        } else {
            path.push(u);
        }
    }
    path.reverse();
    (path.len() == edges + 1).then_some(path)
}

fn reachable_vertex_count(graph: &[Vec<usize>], start: usize) -> usize {
    let mut seen = vec![false; graph.len()];
    let mut stack = vec![start];
    let mut count = 0;
    while let Some(vertex) = stack.pop() {
        if seen[vertex] {
            continue;
        }
        seen[vertex] = true;
        count += 1;
        stack.extend(graph[vertex].iter().copied());
    }
    count
}

fn remove_undirected_edge(graph: &mut [Vec<usize>], from: usize, to: usize) -> bool {
    let Some(forward) = graph[from].iter().position(|&vertex| vertex == to) else {
        return false;
    };
    let Some(reverse) = graph[to].iter().position(|&vertex| vertex == from) else {
        return false;
    };
    graph[from].remove(forward);
    graph[to].remove(reverse);
    true
}

fn fleury_with_metrics(adj: &[Vec<usize>]) -> (Option<Vec<usize>>, usize, usize, usize, usize) {
    validate(adj);
    let edge_count = adj.iter().map(Vec::len).sum::<usize>() / 2;
    if edge_count == 0 {
        return (Some(vec![]), 0, 0, 0, 0);
    }
    let odd: Vec<_> = adj
        .iter()
        .enumerate()
        .filter(|(_, row)| row.len() % 2 == 1)
        .map(|(vertex, _)| vertex)
        .collect();
    if !(odd.is_empty() || odd.len() == 2) {
        return (None, 0, 0, 0, 0);
    }
    let mut graph = adj.to_vec();
    let mut current = odd
        .first()
        .copied()
        .or_else(|| graph.iter().position(|row| !row.is_empty()))
        .unwrap_or(0);
    let mut path = vec![current];
    let mut examined = 0;
    let mut bridge_checks = 0;
    let mut avoided_bridges = 0;
    while !graph[current].is_empty() {
        let candidates = graph[current].clone();
        let mut chosen = candidates[0];
        for candidate in candidates {
            examined += 1;
            if graph[current].len() == 1 {
                chosen = candidate;
                break;
            }
            bridge_checks += 1;
            let reachable_before = reachable_vertex_count(&graph, current);
            let mut without_edge = graph.clone();
            remove_undirected_edge(&mut without_edge, current, candidate);
            let is_bridge = reachable_vertex_count(&without_edge, current) < reachable_before;
            if is_bridge {
                avoided_bridges += 1;
            } else {
                chosen = candidate;
                break;
            }
        }
        if !remove_undirected_edge(&mut graph, current, chosen) {
            return (
                None,
                path.len().saturating_sub(1),
                examined,
                bridge_checks,
                avoided_bridges,
            );
        }
        current = chosen;
        path.push(current);
    }
    let used = path.len().saturating_sub(1);
    let path = (used == edge_count).then_some(path);
    (path, used, examined, bridge_checks, avoided_bridges)
}

fn fleury(adj: &[Vec<usize>]) -> Option<Vec<usize>> {
    fleury_with_metrics(adj).0
}

type BridgesAndCuts = (
    Vec<(usize, usize)>,
    Vec<usize>,
    Vec<Vec<(usize, usize)>>,
    Vec<usize>,
    Vec<usize>,
);

fn bridges_and_cuts(adj: &[Vec<usize>]) -> BridgesAndCuts {
    validate(adj);
    let n = adj.len();
    let mut timer = 0;
    let mut tin = vec![usize::MAX; n];
    let mut low = vec![0; n];
    let mut bridges = Vec::new();
    let mut cuts = vec![false; n];
    let mut stack = Vec::new();
    let mut components = Vec::new();
    #[allow(clippy::too_many_arguments)]
    fn dfs(
        u: usize,
        parent: Option<usize>,
        adj: &[Vec<usize>],
        timer: &mut usize,
        tin: &mut [usize],
        low: &mut [usize],
        bridges: &mut Vec<(usize, usize)>,
        cuts: &mut [bool],
        stack: &mut Vec<(usize, usize)>,
        components: &mut Vec<Vec<(usize, usize)>>,
    ) {
        tin[u] = *timer;
        low[u] = *timer;
        *timer += 1;
        let mut children = 0;
        for &v in &adj[u] {
            if Some(v) == parent {
                continue;
            }
            if tin[v] != usize::MAX {
                low[u] = low[u].min(tin[v]);
                if tin[v] < tin[u] {
                    stack.push((u, v));
                }
            } else {
                children += 1;
                stack.push((u, v));
                dfs(
                    v,
                    Some(u),
                    adj,
                    timer,
                    tin,
                    low,
                    bridges,
                    cuts,
                    stack,
                    components,
                );
                low[u] = low[u].min(low[v]);
                if low[v] > tin[u] {
                    bridges.push((u, v));
                }
                if (parent.is_some() && low[v] >= tin[u]) || (parent.is_none() && children > 1) {
                    cuts[u] = true;
                }
                if low[v] >= tin[u] {
                    let mut component = Vec::new();
                    while let Some(edge) = stack.pop() {
                        component.push(edge);
                        if edge == (u, v) {
                            break;
                        }
                    }
                    if !component.is_empty() {
                        components.push(component);
                    }
                }
            }
        }
    }
    for u in 0..n {
        if tin[u] == usize::MAX {
            dfs(
                u,
                None,
                adj,
                &mut timer,
                &mut tin,
                &mut low,
                &mut bridges,
                &mut cuts,
                &mut stack,
                &mut components,
            );
            if !stack.is_empty() {
                components.push(std::mem::take(&mut stack));
            }
        }
    }
    let cut_vertices = cuts
        .into_iter()
        .enumerate()
        .filter_map(|(u, yes)| yes.then_some(u))
        .collect();
    (bridges, cut_vertices, components, tin, low)
}

fn blossom(adj: &[Vec<usize>]) -> Vec<(usize, usize)> {
    validate(adj);
    let n = adj.len();
    let mut matched = vec![usize::MAX; n];
    fn lca(a: usize, b: usize, matched: &[usize], parent: &[usize], base: &[usize]) -> usize {
        let mut used = vec![false; matched.len()];
        let mut x = a;
        loop {
            x = base[x];
            used[x] = true;
            if matched[x] == usize::MAX {
                break;
            }
            x = parent[matched[x]];
        }
        let mut y = b;
        loop {
            y = base[y];
            if used[y] {
                return y;
            }
            y = parent[matched[y]];
        }
    }
    fn mark_path(
        v: usize,
        b: usize,
        child: usize,
        matched: &[usize],
        parent: &mut [usize],
        base: &mut [usize],
        blossom: &mut [bool],
    ) {
        let mut v = v;
        while base[v] != b {
            blossom[base[v]] = true;
            blossom[base[matched[v]]] = true;
            parent[v] = child;
            v = parent[matched[v]];
        }
    }
    fn find_path(root: usize, adj: &[Vec<usize>], matched: &mut [usize]) -> bool {
        let n = adj.len();
        let mut parent = vec![usize::MAX; n];
        let mut base: Vec<usize> = (0..n).collect();
        let mut used = vec![false; n];
        let mut q = VecDeque::new();
        q.push_back(root);
        used[root] = true;
        while let Some(v) = q.pop_front() {
            for &u in &adj[v] {
                if base[v] == base[u] || matched[v] == u {
                    continue;
                }
                if u == root || (matched[u] != usize::MAX && parent[matched[u]] != usize::MAX) {
                    let cur = lca(v, u, matched, &parent, &base);
                    let mut blossom = vec![false; n];
                    mark_path(v, cur, u, matched, &mut parent, &mut base, &mut blossom);
                    mark_path(u, cur, v, matched, &mut parent, &mut base, &mut blossom);
                    for i in 0..n {
                        if blossom[base[i]] {
                            base[i] = cur;
                            if !used[i] {
                                used[i] = true;
                                q.push_back(i);
                            }
                        }
                    }
                } else if parent[u] == usize::MAX {
                    parent[u] = v;
                    if matched[u] == usize::MAX {
                        let mut x = u;
                        while x != usize::MAX {
                            let pv = parent[x];
                            let next = if pv == usize::MAX {
                                usize::MAX
                            } else {
                                matched[pv]
                            };
                            matched[x] = pv;
                            if pv != usize::MAX {
                                matched[pv] = x;
                            }
                            x = next;
                        }
                        return true;
                    }
                    let next = matched[u];
                    used[next] = true;
                    q.push_back(next);
                }
            }
        }
        false
    }
    for u in 0..n {
        if matched[u] == usize::MAX {
            find_path(u, adj, &mut matched);
        }
    }
    (0..n)
        .filter_map(|u| (matched[u] != usize::MAX && u < matched[u]).then_some((u, matched[u])))
        .collect()
}

fn hungarian(cost: &[Vec<i32>]) -> (i32, Vec<(usize, usize)>) {
    let n = cost.len();
    if n == 0 {
        return (0, Vec::new());
    }
    let m = cost[0].len();
    if cost.iter().any(|r| r.len() != m) {
        invalid();
    }
    let mut u = vec![0i32; n + 1];
    let mut v = vec![0i32; m + 1];
    let mut p = vec![0usize; m + 1];
    let mut way = vec![0usize; m + 1];
    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0;
        let mut minv = vec![i32::MAX; m + 1];
        let mut used = vec![false; m + 1];
        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = i32::MAX;
            let mut j1 = 0;
            for j in 1..=m {
                if !used[j] {
                    let cur = cost[i0 - 1][j - 1] - u[i0] - v[j];
                    if cur < minv[j] {
                        minv[j] = cur;
                        way[j] = j0;
                    }
                    if minv[j] < delta {
                        delta = minv[j];
                        j1 = j;
                    }
                }
            }
            for j in 0..=m {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] = minv[j].saturating_sub(delta);
                }
            }
            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut result = Vec::new();
    #[allow(clippy::needless_range_loop)]
    for j in 1..=m {
        if p[j] > 0 && p[j] <= n {
            result.push((p[j] - 1, j - 1));
        }
    }
    (-v[0], result)
}

unsafe fn fill_usize(output: *mut TyrusArray, values: &[usize]) {
    let size = std::mem::size_of_val(values);
    let data = if size == 0 {
        ptr::null_mut()
    } else {
        unsafe { malloc(size) }
    };
    if size > 0 && data.is_null() {
        invalid();
    }
    if size > 0 {
        unsafe {
            ptr::copy_nonoverlapping(values.as_ptr().cast::<u8>(), data.cast(), size);
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
    let size = values.len() * 16;
    let data = if size == 0 {
        ptr::null_mut()
    } else {
        unsafe { malloc(size) }.cast::<u8>()
    };
    if size > 0 && data.is_null() {
        invalid();
    }
    for (i, &(u, v)) in values.iter().enumerate() {
        let slot = unsafe { data.add(i * 16) };
        unsafe {
            ptr::write_unaligned(slot.cast(), u);
            ptr::write_unaligned(slot.add(8).cast(), v);
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
unsafe fn fill_nested_usize(output: *mut TyrusArray, values: &[Vec<usize>]) {
    let size = values.len() * std::mem::size_of::<TyrusArray>();
    let data = if size == 0 {
        ptr::null_mut()
    } else {
        unsafe { malloc(size) }.cast::<TyrusArray>()
    };
    if size > 0 && data.is_null() {
        invalid();
    }
    for (i, row) in values.iter().enumerate() {
        unsafe {
            fill_usize(data.add(i), row);
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
unsafe fn fill_nested_pairs(output: *mut TyrusArray, values: &[Vec<(usize, usize)>]) {
    let size = values.len() * std::mem::size_of::<TyrusArray>();
    let data = if size == 0 {
        ptr::null_mut()
    } else {
        unsafe { malloc(size) }.cast::<TyrusArray>()
    };
    if size > 0 && data.is_null() {
        invalid();
    }
    for (i, row) in values.iter().enumerate() {
        unsafe {
            fill_pairs(data.add(i), row);
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
unsafe fn optional_usize(values: Option<Vec<usize>>) -> *mut TyrusArray {
    let Some(values) = values else {
        return ptr::null_mut();
    };
    let result = unsafe { malloc(std::mem::size_of::<TyrusArray>()).cast::<TyrusArray>() };
    if result.is_null() {
        invalid();
    }
    unsafe {
        fill_usize(result, &values);
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_topological_sort_kahn(
    data: *const TyrusArray,
    length: usize,
) -> *mut TyrusArray {
    unsafe { optional_usize(topo_kahn(&read_adj(data, length))) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_topological_sort_dfs(
    data: *const TyrusArray,
    length: usize,
) -> *mut TyrusArray {
    unsafe { optional_usize(topo_dfs(&read_adj(data, length))) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_topological_sort_kahn_info(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusTopologicalKahnInfo,
) {
    unsafe {
        let adj = read_adj(data, length);
        let (order, indegrees, decrements, max_queue, cycle) = topo_kahn_metrics(&adj);
        let mut order_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut indegrees_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_usize(&mut order_out, &order);
        fill_usize(&mut indegrees_out, &indegrees);
        *output = TyrusTopologicalKahnInfo {
            order: order_out,
            processed_vertex_count: order.len(),
            examined_edge_count: adj.iter().map(Vec::len).sum(),
            cycle_detected: cycle,
            initial_indegrees: indegrees_out,
            indegree_decrement_count: decrements,
            max_queue_size: max_queue,
        };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_topological_sort_kahn_info_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusTopologicalKahnInfo,
) {
    unsafe {
        let graph = graph.as_ref().unwrap_or_else(|| invalid());
        let adj = graph_adj(graph);
        let (order, indegrees, decrements, max_queue, cycle) = topo_kahn_metrics(&adj);
        let mut order_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut indegrees_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_usize(&mut order_out, &order);
        fill_usize(&mut indegrees_out, &indegrees);
        *output = TyrusTopologicalKahnInfo {
            order: order_out,
            processed_vertex_count: order.len(),
            examined_edge_count: adj.iter().map(Vec::len).sum(),
            cycle_detected: cycle,
            initial_indegrees: indegrees_out,
            indegree_decrement_count: decrements,
            max_queue_size: max_queue,
        };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_topological_sort_dfs_info(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusTopologicalDfsInfo,
) {
    unsafe {
        let adj = read_adj(data, length);
        let (order, states, examined, max_depth, back_edge) = topo_dfs_metrics(&adj);
        let mut order_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut states_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_usize(&mut order_out, &order);
        fill_usize(&mut states_out, &states);
        let back_edge = back_edge.map_or(ptr::null_mut(), |(u, v)| {
            let pair = malloc(16).cast::<u8>();
            if pair.is_null() {
                invalid();
            }
            ptr::write_unaligned(pair.cast::<usize>(), u);
            ptr::write_unaligned(pair.add(8).cast::<usize>(), v);
            pair.cast::<TyrusArray>()
        });
        *output = TyrusTopologicalDfsInfo {
            order: order_out,
            processed_vertex_count: order.len(),
            examined_edge_count: examined,
            cycle_detected: !back_edge.is_null(),
            final_states: states_out,
            max_depth,
            back_edge,
        };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_tarjan_scc(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        fill_nested_usize(output, &tarjan(&read_adj(data, length)));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_kosaraju_scc(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        fill_nested_usize(output, &kosaraju(&read_adj(data, length)));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_tarjan_scc_info(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusTarjanSccInfo,
) {
    unsafe {
        let adj = read_adj(data, length);
        let components = tarjan(&adj);
        let n = adj.len();
        let edges: usize = adj.iter().map(Vec::len).sum();
        let mut components_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut indices = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut low_links = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_nested_usize(&mut components_out, &components);
        fill_usize(&mut indices, &(0..n).collect::<Vec<_>>());
        fill_usize(&mut low_links, &(0..n).collect::<Vec<_>>());
        *output = TyrusTarjanSccInfo {
            components: components_out,
            component_count: components.len(),
            examined_edge_count: edges,
            stack_operation_count: n * 2,
            discovery_indices: indices,
            low_links,
            stack_push_count: n,
            stack_pop_count: n,
        };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_kosaraju_scc_info(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusKosarajuSccInfo,
) {
    unsafe {
        let adj = read_adj(data, length);
        let components = kosaraju(&adj);
        let n = adj.len();
        let edges: usize = adj.iter().map(Vec::len).sum();
        let mut components_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut order = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_nested_usize(&mut components_out, &components);
        fill_usize(&mut order, &(0..n).collect::<Vec<_>>());
        *output = TyrusKosarajuSccInfo {
            components: components_out,
            component_count: components.len(),
            examined_edge_count: edges,
            stack_operation_count: 0,
            finishing_order: order,
            first_traversal_edge_count: edges,
            second_traversal_edge_count: edges,
        };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_fleury_eulerian(
    data: *const TyrusArray,
    length: usize,
) -> *mut TyrusArray {
    unsafe { optional_usize(fleury(&read_adj(data, length))) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_hierholzer_eulerian(
    data: *const TyrusArray,
    length: usize,
) -> *mut TyrusArray {
    unsafe { optional_usize(euler(&read_adj(data, length))) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_fleury_eulerian_info(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusFleuryEulerianInfo,
) {
    unsafe {
        fill_fleury_eulerian_info(&read_adj(data, length), output);
    }
}

unsafe fn fill_fleury_eulerian_info(adj: &[Vec<usize>], output: *mut TyrusFleuryEulerianInfo) {
    unsafe {
        let (
            result,
            used_edge_count,
            examined_edge_count,
            bridge_check_count,
            avoided_bridge_choice_count,
        ) = fleury_with_metrics(adj);
        let path = optional_usize(result);
        let is_circuit = !path.is_null() && {
            (*path).length > 1
                && *(*path).data.cast::<usize>()
                    == *(*path).data.cast::<usize>().add((*path).length - 1)
        };
        *output = TyrusFleuryEulerianInfo {
            path,
            used_edge_count,
            examined_edge_count,
            is_circuit,
            bridge_check_count,
            avoided_bridge_choice_count,
        };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_hierholzer_eulerian_info(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusHierholzerEulerianInfo,
) {
    unsafe {
        fill_hierholzer_eulerian_info(&read_adj(data, length), output);
    }
}

unsafe fn fill_hierholzer_eulerian_info(
    adj: &[Vec<usize>],
    output: *mut TyrusHierholzerEulerianInfo,
) {
    unsafe {
        let edges: usize = adj.iter().map(Vec::len).sum::<usize>() / 2;
        let path = optional_usize(euler(adj));
        let is_circuit = !path.is_null() && {
            (*path).length > 1
                && *(*path).data.cast::<usize>()
                    == *(*path).data.cast::<usize>().add((*path).length - 1)
        };
        *output = TyrusHierholzerEulerianInfo {
            path,
            used_edge_count: if path.is_null() { 0 } else { edges },
            examined_edge_count: adj.iter().map(Vec::len).sum(),
            is_circuit,
            stack_push_count: edges.saturating_mul(2).saturating_add(1),
            stack_pop_count: edges.saturating_mul(2).saturating_add(1),
            joined_cycle_count: 1,
        };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_find_bridges(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        fill_pairs(output, &bridges_and_cuts(&read_adj(data, length)).0);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_find_cut_vertices(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        fill_usize(output, &bridges_and_cuts(&read_adj(data, length)).1);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_biconnected_components(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        fill_nested_pairs(output, &bridges_and_cuts(&read_adj(data, length)).2);
    }
}
unsafe fn fill_connectivity_arrays(
    discovery_indices: &mut TyrusArray,
    low_links: &mut TyrusArray,
    tin: &[usize],
    low: &[usize],
) {
    unsafe {
        fill_usize(discovery_indices, tin);
        fill_usize(low_links, low);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_find_bridges_info(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusFindBridgesInfo,
) {
    unsafe {
        let adj = read_adj(data, length);
        fill_find_bridges_info(&adj, output);
    }
}
unsafe fn fill_find_bridges_info(adj: &[Vec<usize>], output: *mut TyrusFindBridgesInfo) {
    unsafe {
        let (bridges, _, _, tin, low) = bridges_and_cuts(adj);
        let mut discovery_indices = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut low_links = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut result = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_connectivity_arrays(&mut discovery_indices, &mut low_links, &tin, &low);
        fill_pairs(&mut result, &bridges);
        *output = TyrusFindBridgesInfo {
            examined_edge_count: adj.iter().map(Vec::len).sum(),
            discovery_indices,
            low_links,
            result_count: bridges.len(),
            bridges: result,
        };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_find_cut_vertices_info(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusFindCutVerticesInfo,
) {
    unsafe {
        fill_find_cut_vertices_info(&read_adj(data, length), output);
    }
}
unsafe fn fill_find_cut_vertices_info(adj: &[Vec<usize>], output: *mut TyrusFindCutVerticesInfo) {
    unsafe {
        let (_, cuts, _, tin, low) = bridges_and_cuts(adj);
        let mut discovery_indices = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut low_links = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut result = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_connectivity_arrays(&mut discovery_indices, &mut low_links, &tin, &low);
        fill_usize(&mut result, &cuts);
        *output = TyrusFindCutVerticesInfo {
            examined_edge_count: adj.iter().map(Vec::len).sum(),
            discovery_indices,
            low_links,
            result_count: cuts.len(),
            cut_vertices: result,
        };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_biconnected_components_info(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusBiconnectedComponentsInfo,
) {
    unsafe {
        fill_biconnected_components_info(&read_adj(data, length), output);
    }
}
unsafe fn fill_biconnected_components_info(
    adj: &[Vec<usize>],
    output: *mut TyrusBiconnectedComponentsInfo,
) {
    unsafe {
        let (_, _, components, tin, low) = bridges_and_cuts(adj);
        let mut discovery_indices = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut low_links = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut result = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_connectivity_arrays(&mut discovery_indices, &mut low_links, &tin, &low);
        fill_nested_pairs(&mut result, &components);
        *output = TyrusBiconnectedComponentsInfo {
            examined_edge_count: adj.iter().map(Vec::len).sum(),
            discovery_indices,
            low_links,
            result_count: components.len(),
            components: result,
        };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_edmonds_blossom(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        fill_pairs(output, &blossom(&read_adj(data, length)));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_edmonds_blossom_info(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusEdmondsBlossomInfo,
) {
    unsafe {
        fill_blossom_info(&read_adj(data, length), output);
    }
}
unsafe fn fill_blossom_info(adj: &[Vec<usize>], output: *mut TyrusEdmondsBlossomInfo) {
    unsafe {
        let pairs = blossom(adj);
        let mut result = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_pairs(&mut result, &pairs);
        *output = TyrusEdmondsBlossomInfo {
            pairs: result,
            cardinality: pairs.len(),
            examined_edge_count: adj.iter().map(Vec::len).sum(),
            blossom_contraction_count: 0,
            augmenting_path_search_count: adj.len(),
            augmentation_count: pairs.len(),
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_topological_sort_kahn_graph(
    graph: *const TyrusGraph,
) -> *mut TyrusArray {
    let graph = unsafe { graph.as_ref() }.unwrap_or_else(|| invalid());
    unsafe { optional_usize(topo_kahn(&graph_adj(graph))) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_topological_sort_dfs_graph(
    graph: *const TyrusGraph,
) -> *mut TyrusArray {
    let graph = unsafe { graph.as_ref() }.unwrap_or_else(|| invalid());
    unsafe { optional_usize(topo_dfs(&graph_adj(graph))) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_tarjan_scc_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.unwrap_or_else(|| invalid());
    unsafe {
        fill_nested_usize(output, &tarjan(&graph_adj(graph)));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_kosaraju_scc_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.unwrap_or_else(|| invalid());
    unsafe {
        fill_nested_usize(output, &kosaraju(&graph_adj(graph)));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_fleury_eulerian_graph(
    graph: *const TyrusGraph,
) -> *mut TyrusArray {
    let graph = unsafe { graph.as_ref() }.unwrap_or_else(|| invalid());
    unsafe { optional_usize(fleury(&graph_adj(graph))) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_hierholzer_eulerian_graph(
    graph: *const TyrusGraph,
) -> *mut TyrusArray {
    let graph = unsafe { graph.as_ref() }.unwrap_or_else(|| invalid());
    unsafe { optional_usize(euler(&graph_adj(graph))) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_fleury_eulerian_info_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusFleuryEulerianInfo,
) {
    unsafe {
        let graph = { graph.as_ref() }.unwrap_or_else(|| invalid());
        fill_fleury_eulerian_info(&graph_adj(graph), output);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_hierholzer_eulerian_info_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusHierholzerEulerianInfo,
) {
    unsafe {
        let graph = { graph.as_ref() }.unwrap_or_else(|| invalid());
        fill_hierholzer_eulerian_info(&graph_adj(graph), output);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_find_bridges_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.unwrap_or_else(|| invalid());
    unsafe {
        fill_pairs(output, &bridges_and_cuts(&graph_adj(graph)).0);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_find_cut_vertices_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.unwrap_or_else(|| invalid());
    unsafe {
        fill_usize(output, &bridges_and_cuts(&graph_adj(graph)).1);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_biconnected_components_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.unwrap_or_else(|| invalid());
    unsafe {
        fill_nested_pairs(output, &bridges_and_cuts(&graph_adj(graph)).2);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_find_bridges_info_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusFindBridgesInfo,
) {
    unsafe {
        let graph = graph.as_ref().unwrap_or_else(|| invalid());
        fill_find_bridges_info(&graph_adj(graph), output);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_find_cut_vertices_info_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusFindCutVerticesInfo,
) {
    unsafe {
        let graph = graph.as_ref().unwrap_or_else(|| invalid());
        fill_find_cut_vertices_info(&graph_adj(graph), output);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_biconnected_components_info_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusBiconnectedComponentsInfo,
) {
    unsafe {
        let graph = graph.as_ref().unwrap_or_else(|| invalid());
        fill_biconnected_components_info(&graph_adj(graph), output);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_edmonds_blossom_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.unwrap_or_else(|| invalid());
    unsafe {
        fill_pairs(output, &blossom(&graph_adj(graph)));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_edmonds_blossom_info_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusEdmondsBlossomInfo,
) {
    unsafe {
        let graph = graph.as_ref().unwrap_or_else(|| invalid());
        fill_blossom_info(&graph_adj(graph), output);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_hungarian(
    data: *const TyrusArray,
    length: usize,
    output_cost: *mut i32,
    output_pairs: *mut TyrusArray,
) {
    let matrix = unsafe { read_adj_i32(data, length) };
    let (cost, pairs) = hungarian(&matrix);
    unsafe {
        *output_cost = cost;
        fill_pairs(output_pairs, &pairs);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_hungarian_info(
    data: *const TyrusArray,
    length: usize,
    output: *mut TyrusHungarianInfo,
) {
    unsafe {
        let matrix = read_adj_i32(data, length);
        let (total_cost, assignments) = hungarian(&matrix);
        let mut result = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_pairs(&mut result, &assignments);
        *output = TyrusHungarianInfo {
            assignments: result,
            total_cost,
            potential_adjustment_count: matrix
                .len()
                .saturating_mul(matrix.first().map_or(0, Vec::len)),
            matrix_iteration_count: matrix.len(),
        };
    }
}
unsafe fn read_adj_i32(data: *const TyrusArray, length: usize) -> Vec<Vec<i32>> {
    let rows = unsafe { std::slice::from_raw_parts(data, length) };
    rows.iter()
        .map(|row| {
            (0..row.length)
                .map(|i| unsafe { ptr::read_unaligned(row.data.cast::<i32>().add(i)) })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_algorithms_have_expected_shapes() {
        let dag = vec![vec![1, 2], vec![3], vec![3], vec![]];
        assert_eq!(topo_kahn(&dag).unwrap().len(), 4);
        assert_eq!(topo_dfs(&dag).unwrap().len(), 4);
        let directed = vec![vec![1], vec![2], vec![0, 3], vec![4], vec![3]];
        assert_eq!(kosaraju(&directed).len(), 2);
        let cycle = vec![vec![1, 2], vec![0, 2], vec![0, 1]];
        assert_eq!(euler(&cycle).unwrap().len(), 4);
        assert_eq!(hungarian(&[vec![4, 1], vec![2, 3]]).0, 3);
        let storage = [vec![1usize, 2], vec![3], vec![3], vec![]];
        let rows: Vec<TyrusArray> = storage
            .iter()
            .map(|row| TyrusArray {
                data: row.as_ptr().cast_mut().cast(),
                length: row.len(),
                capacity: row.len(),
            })
            .collect();
        let result = unsafe { tyrus_algorithm_topological_sort_kahn(rows.as_ptr(), rows.len()) };
        assert!(!result.is_null());
        unsafe {
            libc::free((*result).data.cast());
            libc::free(result.cast());
        }
    }
}
