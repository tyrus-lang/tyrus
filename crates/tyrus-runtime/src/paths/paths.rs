use std::cell::Cell;
use std::collections::VecDeque;
use std::ffi::{CString, c_void};
use std::ptr;

use crate::{TyrusArray, TyrusGraph, tyrus_panic};
use tyrus_string::TyrusString;

#[repr(C)]
pub struct TyrusTraversalInfo {
    pub start: usize,
    pub visit_order: TyrusArray,
    pub visited_count: usize,
    pub examined_edges: usize,
    pub max_frontier_size: usize,
    pub max_depth: usize,
}

#[repr(C)]
pub struct TyrusIddfsInfo {
    pub start: usize,
    pub visit_order: TyrusArray,
    pub visited_count: usize,
    pub examined_edges: usize,
    pub max_frontier_size: usize,
    pub max_depth: usize,
    pub depth_limits_tried: TyrusArray,
    pub iterations: usize,
}

#[repr(C)]
pub struct TyrusDijkstraInfo {
    pub distances: TyrusArray,
    pub predecessors: TyrusArray,
    pub relaxation_count: usize,
    pub processed_vertex_count: usize,
    pub examined_edge_count: usize,
    pub heap_extraction_count: usize,
    pub accepted_relaxation_count: usize,
    pub finalized_vertex_count: usize,
}

#[repr(C)]
pub struct TyrusAStarInfo {
    pub distances: TyrusArray,
    pub predecessors: TyrusArray,
    pub relaxation_count: usize,
    pub processed_vertex_count: usize,
    pub examined_edge_count: usize,
    pub heuristic_evaluation_count: usize,
    pub opened_vertex_count: usize,
    pub closed_vertex_count: usize,
    pub final_cost: *mut i32,
}

#[repr(C)]
pub struct TyrusWeightedEdge {
    pub from: usize,
    pub to: usize,
    pub weight: i32,
}

#[repr(C)]
pub struct TyrusBellmanFordInfo {
    pub distances: TyrusArray,
    pub predecessors: TyrusArray,
    pub relaxation_count: usize,
    pub processed_vertex_count: usize,
    pub examined_edge_count: usize,
    pub pass_count: usize,
    pub negative_cycle_detected: bool,
    pub negative_cycle_edge: *mut TyrusWeightedEdge,
}

#[repr(C)]
pub struct TyrusSpfaInfo {
    pub distances: TyrusArray,
    pub predecessors: TyrusArray,
    pub relaxation_count: usize,
    pub processed_vertex_count: usize,
    pub examined_edge_count: usize,
    pub queue_push_count: usize,
    pub queue_pop_count: usize,
    pub max_queue_size: usize,
}

#[repr(C)]
pub struct TyrusJohnsonInfo {
    pub distances: TyrusArray,
    pub reweighting_count: usize,
    pub bellman_ford_relaxation_count: usize,
    pub dijkstra_execution_count: usize,
}

#[repr(C)]
pub struct TyrusFloydWarshallInfo {
    pub distances: TyrusArray,
    pub k_pass_count: usize,
    pub distance_update_count: usize,
    pub negative_cycle_detected: bool,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

fn panic_out_of_bounds(what: &str) -> ! {
    let message = CString::new(format!("tyrus runtime error: {what} is out of bounds")).unwrap();
    unsafe { tyrus_panic(message.as_ptr()) }
}

fn allocation_failed() -> ! {
    unsafe { crate::tyrus_panic(c"tyrus runtime error: path algorithm allocation failed".as_ptr()) }
}

#[derive(Debug)]
struct NegativeCycle;

thread_local! {
    static LAST_ALGORITHM_ERROR: Cell<Option<(&'static str, &'static str)>> = const {
        Cell::new(None)
    };
}

fn set_algorithm_error(code: &'static str, message: &'static str) {
    LAST_ALGORITHM_ERROR.with(|slot| slot.set(Some((code, message))));
}

#[repr(C)]
pub struct AlgorithmErrorTaken {
    pub code: TyrusString,
    pub message: TyrusString,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_take_error(out: *mut AlgorithmErrorTaken) {
    LAST_ALGORITHM_ERROR.with(|slot| {
        if let Some((code, message)) = slot.take()
            && !out.is_null()
        {
            unsafe {
                (*out).code = TyrusString::literal(code);
                (*out).message = TyrusString::literal(message);
            }
        }
    });
}

fn bfs(adj: &[Vec<usize>], start: usize) -> Vec<usize> {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    let mut visited = vec![false; n];
    let mut queue = VecDeque::new();
    let mut order = Vec::new();
    visited[start] = true;
    queue.push_back(start);
    while let Some(u) = queue.pop_front() {
        order.push(u);
        for &v in &adj[u] {
            if !visited[v] {
                visited[v] = true;
                queue.push_back(v);
            }
        }
    }
    order
}

fn bfs_info(adj: &[Vec<usize>], start: usize) -> (Vec<usize>, usize, usize, usize) {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    let (mut visited, mut queue, mut order, mut depths) =
        (vec![false; n], VecDeque::new(), Vec::new(), vec![0usize; n]);
    let (mut examined, mut max_frontier, mut max_depth) = (0, 1, 0);
    visited[start] = true;
    queue.push_back(start);
    while let Some(u) = queue.pop_front() {
        order.push(u);
        for &v in &adj[u] {
            examined += 1;
            if !visited[v] {
                visited[v] = true;
                depths[v] = depths[u] + 1;
                max_depth = max_depth.max(depths[v]);
                queue.push_back(v);
                max_frontier = max_frontier.max(queue.len());
            }
        }
    }
    (order, examined, max_frontier, max_depth)
}

fn dfs(adj: &[Vec<usize>], start: usize) -> Vec<usize> {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    let mut visited = vec![false; n];
    let mut stack = vec![start];
    let mut order = Vec::new();
    while let Some(u) = stack.pop() {
        if !visited[u] {
            visited[u] = true;
            order.push(u);
            for &v in adj[u].iter().rev() {
                if !visited[v] {
                    stack.push(v);
                }
            }
        }
    }
    order
}

fn dfs_info(adj: &[Vec<usize>], start: usize) -> (Vec<usize>, usize, usize, usize) {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    let (mut visited, mut stack, mut order) = (vec![false; n], vec![(start, 0usize)], Vec::new());
    let (mut examined, mut max_frontier, mut max_depth) = (0, 1, 0);
    while let Some((u, depth)) = stack.pop() {
        if !visited[u] {
            visited[u] = true;
            order.push(u);
            max_depth = max_depth.max(depth);
            for &v in adj[u].iter().rev() {
                examined += 1;
                if !visited[v] {
                    stack.push((v, depth + 1));
                    max_frontier = max_frontier.max(stack.len());
                }
            }
        }
    }
    (order, examined, max_frontier, max_depth)
}

fn iddfs(adj: &[Vec<usize>], start: usize, max_depth: usize) -> Vec<usize> {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    let mut recorded = vec![false; n];
    let mut order = Vec::new();
    let mut depth_limit = 0usize;
    while order.len() < n && depth_limit <= max_depth {
        let mut pass_seen = vec![false; n];
        let mut stack = vec![(start, 0usize)];
        while let Some((u, depth)) = stack.pop() {
            if pass_seen[u] {
                continue;
            }
            pass_seen[u] = true;
            if !recorded[u] {
                recorded[u] = true;
                order.push(u);
            }
            if depth < depth_limit {
                for &v in adj[u].iter().rev() {
                    if !pass_seen[v] {
                        stack.push((v, depth + 1));
                    }
                }
            }
        }
        depth_limit += 1;
    }
    order
}

fn iddfs_info(
    adj: &[Vec<usize>],
    start: usize,
    max_depth: usize,
) -> (Vec<usize>, usize, usize, usize, Vec<usize>) {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    let (mut recorded, mut order, mut limits) = (vec![false; n], Vec::new(), Vec::new());
    let (mut examined, mut max_frontier, mut observed_depth) = (0, 0, 0);
    for limit in 0..=max_depth {
        limits.push(limit);
        let mut seen = vec![false; n];
        let mut stack = vec![(start, 0usize)];
        max_frontier = max_frontier.max(1);
        while let Some((u, depth)) = stack.pop() {
            if seen[u] {
                continue;
            }
            seen[u] = true;
            observed_depth = observed_depth.max(depth);
            if !recorded[u] {
                recorded[u] = true;
                order.push(u);
            }
            if depth < limit {
                for &v in adj[u].iter().rev() {
                    examined += 1;
                    if !seen[v] {
                        stack.push((v, depth + 1));
                        max_frontier = max_frontier.max(stack.len());
                    }
                }
            }
        }
        if order.len() == n {
            break;
        }
    }
    (order, examined, max_frontier, observed_depth, limits)
}

fn dijkstra(adj: &[Vec<(usize, i32)>], start: usize, end: usize) -> Option<(Vec<usize>, i32)> {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    if end >= n {
        panic_out_of_bounds("end");
    }
    let mut dist = vec![i32::MAX; n];
    let mut prev = vec![None; n];
    let mut heap = std::collections::BinaryHeap::new();
    dist[start] = 0;
    heap.push(std::cmp::Reverse((0i32, start)));
    while let Some(std::cmp::Reverse((d, u))) = heap.pop() {
        if d > dist[u] {
            continue;
        }
        if u == end {
            let mut path = Vec::new();
            let mut cur = end;
            while let Some(p) = prev[cur] {
                path.push(cur);
                cur = p;
            }
            path.push(start);
            path.reverse();
            return Some((path, d));
        }
        for &(v, w) in &adj[u] {
            let nd = d + w;
            if nd < dist[v] {
                dist[v] = nd;
                prev[v] = Some(u);
                heap.push(std::cmp::Reverse((nd, v)));
            }
        }
    }
    None
}

fn a_star(
    adj: &[Vec<(usize, i32)>],
    start: usize,
    end: usize,
    h: &[i32],
) -> Option<(Vec<usize>, i32)> {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    if end >= n {
        panic_out_of_bounds("end");
    }
    let h = if h.len() >= n {
        h
    } else {
        if n == 0 {
            return None;
        }
        let mut owned = vec![0i32; n];
        owned[..h.len()].copy_from_slice(h);
        return a_star_with_owned_h(adj, start, end, &owned);
    };
    let mut dist = vec![i32::MAX; n];
    let mut prev = vec![None; n];
    let mut heap = std::collections::BinaryHeap::new();
    dist[start] = 0;
    heap.push(std::cmp::Reverse((h[start], 0i32, start)));
    while let Some(std::cmp::Reverse((_f, d, u))) = heap.pop() {
        if d > dist[u] {
            continue;
        }
        if u == end {
            let mut path = Vec::new();
            let mut cur = end;
            while let Some(p) = prev[cur] {
                path.push(cur);
                cur = p;
            }
            path.push(start);
            path.reverse();
            return Some((path, d));
        }
        for &(v, w) in &adj[u] {
            let nd = d + w;
            if nd < dist[v] {
                dist[v] = nd;
                prev[v] = Some(u);
                heap.push(std::cmp::Reverse((nd + h[v], nd, v)));
            }
        }
    }
    None
}

fn a_star_with_owned_h(
    adj: &[Vec<(usize, i32)>],
    start: usize,
    end: usize,
    h: &[i32],
) -> Option<(Vec<usize>, i32)> {
    let n = adj.len();
    let mut dist = vec![i32::MAX; n];
    let mut prev = vec![None; n];
    let mut heap = std::collections::BinaryHeap::new();
    dist[start] = 0;
    heap.push(std::cmp::Reverse((h[start], 0i32, start)));
    while let Some(std::cmp::Reverse((_f, d, u))) = heap.pop() {
        if d > dist[u] {
            continue;
        }
        if u == end {
            let mut path = Vec::new();
            let mut cur = end;
            while let Some(p) = prev[cur] {
                path.push(cur);
                cur = p;
            }
            path.push(start);
            path.reverse();
            return Some((path, d));
        }
        for &(v, w) in &adj[u] {
            let nd = d + w;
            if nd < dist[v] {
                dist[v] = nd;
                prev[v] = Some(u);
                heap.push(std::cmp::Reverse((nd + h[v], nd, v)));
            }
        }
    }
    None
}

struct ShortestPathStudy {
    distances: Vec<i32>,
    predecessors: Vec<Option<usize>>,
    relaxation_count: usize,
    processed_vertex_count: usize,
    examined_edge_count: usize,
}

struct DijkstraStudy {
    base: ShortestPathStudy,
    heap_extraction_count: usize,
    finalized_vertex_count: usize,
}

fn dijkstra_info(adj: &[Vec<(usize, i32)>], start: usize, end: usize) -> DijkstraStudy {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    if end >= n {
        panic_out_of_bounds("end");
    }
    let (mut distances, mut predecessors, mut finalized) =
        (vec![i32::MAX; n], vec![None; n], vec![false; n]);
    let mut heap = std::collections::BinaryHeap::new();
    let (mut extractions, mut relaxations, mut examined, mut finalized_count) = (0, 0, 0, 0);
    distances[start] = 0;
    heap.push(std::cmp::Reverse((0i32, start)));
    while let Some(std::cmp::Reverse((distance, vertex))) = heap.pop() {
        extractions += 1;
        if distance > distances[vertex] || finalized[vertex] {
            continue;
        }
        finalized[vertex] = true;
        finalized_count += 1;
        if vertex == end {
            break;
        }
        for &(neighbor, weight) in &adj[vertex] {
            examined += 1;
            let candidate = distance + weight;
            if candidate < distances[neighbor] {
                distances[neighbor] = candidate;
                predecessors[neighbor] = Some(vertex);
                relaxations += 1;
                heap.push(std::cmp::Reverse((candidate, neighbor)));
            }
        }
    }
    DijkstraStudy {
        base: ShortestPathStudy {
            distances,
            predecessors,
            relaxation_count: relaxations,
            processed_vertex_count: finalized_count,
            examined_edge_count: examined,
        },
        heap_extraction_count: extractions,
        finalized_vertex_count: finalized_count,
    }
}

struct AStarStudy {
    base: ShortestPathStudy,
    heuristic_evaluation_count: usize,
    opened_vertex_count: usize,
    closed_vertex_count: usize,
    final_cost: Option<i32>,
}

fn a_star_info(
    adj: &[Vec<(usize, i32)>],
    start: usize,
    end: usize,
    heuristic: &[i32],
) -> AStarStudy {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    if end >= n {
        panic_out_of_bounds("end");
    }
    let mut h = vec![0i32; n];
    let copied = heuristic.len().min(n);
    h[..copied].copy_from_slice(&heuristic[..copied]);
    let (mut distances, mut predecessors) = (vec![i32::MAX; n], vec![None; n]);
    let (mut opened, mut closed) = (vec![false; n], vec![false; n]);
    let mut heap = std::collections::BinaryHeap::new();
    let (mut relaxations, mut examined, mut heuristic_evaluations) = (0, 0, 1);
    let (mut opened_count, mut closed_count, mut final_cost) = (1, 0, None);
    distances[start] = 0;
    opened[start] = true;
    heap.push(std::cmp::Reverse((h[start], 0i32, start)));
    while let Some(std::cmp::Reverse((_estimate, distance, vertex))) = heap.pop() {
        if distance > distances[vertex] || closed[vertex] {
            continue;
        }
        closed[vertex] = true;
        closed_count += 1;
        if vertex == end {
            final_cost = Some(distance);
            break;
        }
        for &(neighbor, weight) in &adj[vertex] {
            examined += 1;
            let candidate = distance + weight;
            if candidate < distances[neighbor] {
                distances[neighbor] = candidate;
                predecessors[neighbor] = Some(vertex);
                relaxations += 1;
                heuristic_evaluations += 1;
                if !opened[neighbor] {
                    opened[neighbor] = true;
                    opened_count += 1;
                }
                heap.push(std::cmp::Reverse((
                    candidate + h[neighbor],
                    candidate,
                    neighbor,
                )));
            }
        }
    }
    AStarStudy {
        base: ShortestPathStudy {
            distances,
            predecessors,
            relaxation_count: relaxations,
            processed_vertex_count: closed_count,
            examined_edge_count: examined,
        },
        heuristic_evaluation_count: heuristic_evaluations,
        opened_vertex_count: opened_count,
        closed_vertex_count: closed_count,
        final_cost,
    }
}

struct BellmanFordStudy {
    base: ShortestPathStudy,
    pass_count: usize,
    negative_cycle_edge: Option<(usize, usize, i32)>,
}

fn bellman_ford_info(adj: &[Vec<(usize, i32)>], start: usize) -> BellmanFordStudy {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    let (mut distances, mut predecessors, mut processed) =
        (vec![i32::MAX; n], vec![None; n], vec![false; n]);
    let (mut relaxations, mut examined, mut pass_count) = (0, 0, 0);
    distances[start] = 0;
    for _ in 0..n.saturating_sub(1) {
        pass_count += 1;
        let mut changed = false;
        for vertex in 0..n {
            if distances[vertex] == i32::MAX {
                continue;
            }
            processed[vertex] = true;
            for &(neighbor, weight) in &adj[vertex] {
                examined += 1;
                let candidate = distances[vertex] + weight;
                if candidate < distances[neighbor] {
                    distances[neighbor] = candidate;
                    predecessors[neighbor] = Some(vertex);
                    relaxations += 1;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let mut negative_cycle_edge = None;
    'cycle: for vertex in 0..n {
        if distances[vertex] == i32::MAX {
            continue;
        }
        for &(neighbor, weight) in &adj[vertex] {
            examined += 1;
            if distances[vertex] + weight < distances[neighbor] {
                negative_cycle_edge = Some((vertex, neighbor, weight));
                break 'cycle;
            }
        }
    }
    BellmanFordStudy {
        base: ShortestPathStudy {
            distances,
            predecessors,
            relaxation_count: relaxations,
            processed_vertex_count: processed.into_iter().filter(|value| *value).count(),
            examined_edge_count: examined,
        },
        pass_count,
        negative_cycle_edge,
    }
}

struct SpfaStudy {
    base: ShortestPathStudy,
    queue_push_count: usize,
    queue_pop_count: usize,
    max_queue_size: usize,
}

fn spfa_info(adj: &[Vec<(usize, i32)>], start: usize) -> Result<SpfaStudy, NegativeCycle> {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    let (mut distances, mut predecessors) = (vec![i32::MAX; n], vec![None; n]);
    let (mut in_queue, mut push_counts, mut processed) =
        (vec![false; n], vec![0usize; n], vec![false; n]);
    let mut queue = VecDeque::new();
    let (mut pushes, mut pops, mut max_queue, mut relaxations, mut examined) = (1, 0, 1, 0, 0);
    distances[start] = 0;
    in_queue[start] = true;
    push_counts[start] = 1;
    queue.push_back(start);
    while let Some(vertex) = queue.pop_front() {
        pops += 1;
        processed[vertex] = true;
        in_queue[vertex] = false;
        for &(neighbor, weight) in &adj[vertex] {
            examined += 1;
            let candidate = distances[vertex] + weight;
            if candidate < distances[neighbor] {
                distances[neighbor] = candidate;
                predecessors[neighbor] = Some(vertex);
                relaxations += 1;
                if !in_queue[neighbor] {
                    queue.push_back(neighbor);
                    in_queue[neighbor] = true;
                    pushes += 1;
                    push_counts[neighbor] += 1;
                    max_queue = max_queue.max(queue.len());
                    if push_counts[neighbor] > n {
                        return Err(NegativeCycle);
                    }
                }
            }
        }
    }
    Ok(SpfaStudy {
        base: ShortestPathStudy {
            distances,
            predecessors,
            relaxation_count: relaxations,
            processed_vertex_count: processed.into_iter().filter(|value| *value).count(),
            examined_edge_count: examined,
        },
        queue_push_count: pushes,
        queue_pop_count: pops,
        max_queue_size: max_queue,
    })
}

fn bellman_ford(
    adj: &[Vec<(usize, i32)>],
    start: usize,
) -> Result<(Vec<i32>, Vec<Option<usize>>), NegativeCycle> {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    let mut dist = vec![i32::MAX; n];
    let mut prev = vec![None; n];
    dist[start] = 0;
    for _ in 0..n.saturating_sub(1) {
        let mut relaxed = false;
        for u in 0..n {
            if dist[u] == i32::MAX {
                continue;
            }
            for &(v, w) in &adj[u] {
                let nd = dist[u] + w;
                if nd < dist[v] {
                    dist[v] = nd;
                    prev[v] = Some(u);
                    relaxed = true;
                }
            }
        }
        if !relaxed {
            break;
        }
    }
    for u in 0..n {
        if dist[u] == i32::MAX {
            continue;
        }
        for &(v, w) in &adj[u] {
            if dist[u] + w < dist[v] {
                return Err(NegativeCycle);
            }
        }
    }
    Ok((dist, prev))
}

fn spfa(
    adj: &[Vec<(usize, i32)>],
    start: usize,
) -> Result<(Vec<i32>, Vec<Option<usize>>), NegativeCycle> {
    let n = adj.len();
    if start >= n {
        panic_out_of_bounds("start");
    }
    let mut dist = vec![i32::MAX; n];
    let mut prev = vec![None; n];
    let mut in_queue = vec![false; n];
    let mut count = vec![0usize; n];
    let mut queue = VecDeque::new();
    dist[start] = 0;
    queue.push_back(start);
    in_queue[start] = true;
    count[start] = 1;
    while let Some(u) = queue.pop_front() {
        in_queue[u] = false;
        for &(v, w) in &adj[u] {
            let nd = dist[u] + w;
            if nd < dist[v] {
                dist[v] = nd;
                prev[v] = Some(u);
                if !in_queue[v] {
                    queue.push_back(v);
                    in_queue[v] = true;
                    count[v] += 1;
                    if count[v] > n {
                        return Err(NegativeCycle);
                    }
                }
            }
        }
    }
    Ok((dist, prev))
}

#[allow(clippy::needless_range_loop)]
fn floyd_warshall(adj: &[Vec<Option<i32>>]) -> Vec<Vec<Option<i32>>> {
    let n = adj.len();
    let mut dist = adj.to_vec();
    for k in 0..n {
        for i in 0..n {
            let Some(dik) = dist[i][k] else {
                continue;
            };
            for j in 0..n {
                let Some(dkj) = dist[k][j] else {
                    continue;
                };
                let via_k = dik + dkj;
                dist[i][j] = Some(match dist[i][j] {
                    Some(dij) if dij < via_k => dij,
                    _ => via_k,
                });
            }
        }
    }
    dist
}

#[allow(clippy::needless_range_loop)]
fn floyd_warshall_info(adj: &[Vec<Option<i32>>]) -> (Vec<Vec<Option<i32>>>, usize, usize, bool) {
    let n = adj.len();
    let mut distances = adj.to_vec();
    let mut updates = 0usize;
    for intermediate in 0..n {
        for from in 0..n {
            let Some(left) = distances[from][intermediate] else {
                continue;
            };
            for to in 0..n {
                let Some(right) = distances[intermediate][to] else {
                    continue;
                };
                let candidate = left + right;
                if distances[from][to].is_none_or(|current| candidate < current) {
                    distances[from][to] = Some(candidate);
                    updates += 1;
                }
            }
        }
    }
    let negative_cycle = (0..n).any(|vertex| distances[vertex][vertex].is_some_and(|d| d < 0));
    (distances, n, updates, negative_cycle)
}

fn johnson(adj: &[Vec<(usize, i32)>]) -> Result<Vec<Vec<Option<i32>>>, NegativeCycle> {
    let n = adj.len();
    let mut edges = Vec::new();
    for (u, neighbors) in adj.iter().enumerate() {
        for &(v, w) in neighbors {
            edges.push((u, v, w));
        }
    }
    let mut aug_edges = edges.clone();
    for v in 0..n {
        aug_edges.push((n, v, 0));
    }
    let (h, _) = bellman_ford_from_edges(&aug_edges, n + 1, n)?;
    let reweighted: Vec<(usize, usize, i32)> = edges
        .iter()
        .map(|&(u, v, w)| (u, v, w + h[u] - h[v]))
        .collect();
    let mut result = vec![vec![None; n]; n];
    for s in 0..n {
        let mut d = vec![i32::MAX; n];
        d[s] = 0;
        let mut heap = std::collections::BinaryHeap::new();
        heap.push(std::cmp::Reverse((0i32, s)));
        while let Some(std::cmp::Reverse((du, u))) = heap.pop() {
            if du > d[u] {
                continue;
            }
            for &(_x, v, w) in reweighted.iter().filter(|&&(x, _, _)| x == u) {
                let nd = du + w;
                if nd < d[v] {
                    d[v] = nd;
                    heap.push(std::cmp::Reverse((nd, v)));
                }
            }
        }
        for v in 0..n {
            if d[v] < i32::MAX {
                result[s][v] = Some(d[v] - h[s] + h[v]);
            }
        }
    }
    Ok(result)
}

type JohnsonInfo = (Vec<Vec<Option<i32>>>, usize, usize, usize);

fn johnson_info(adj: &[Vec<(usize, i32)>]) -> Result<JohnsonInfo, NegativeCycle> {
    let n = adj.len();
    let mut edges = Vec::new();
    for (from, neighbors) in adj.iter().enumerate() {
        for &(to, weight) in neighbors {
            edges.push((from, to, weight));
        }
    }
    let mut augmented = edges.clone();
    for vertex in 0..n {
        augmented.push((n, vertex, 0));
    }
    let mut potentials = vec![i32::MAX; n + 1];
    potentials[n] = 0;
    let mut bellman_ford_relaxations = 0usize;
    for _ in 0..n {
        let mut changed = false;
        for &(from, to, weight) in &augmented {
            if potentials[from] != i32::MAX && potentials[from] + weight < potentials[to] {
                potentials[to] = potentials[from] + weight;
                bellman_ford_relaxations += 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for &(from, to, weight) in &augmented {
        if potentials[from] != i32::MAX && potentials[from] + weight < potentials[to] {
            return Err(NegativeCycle);
        }
    }
    let reweighted: Vec<Vec<(usize, i32)>> = adj
        .iter()
        .enumerate()
        .map(|(from, neighbors)| {
            neighbors
                .iter()
                .map(|&(to, weight)| (to, weight + potentials[from] - potentials[to]))
                .collect()
        })
        .collect();
    let mut result = vec![vec![None; n]; n];
    for source in 0..n {
        let mut distances = vec![i32::MAX; n];
        let mut heap = std::collections::BinaryHeap::new();
        distances[source] = 0;
        heap.push(std::cmp::Reverse((0i32, source)));
        while let Some(std::cmp::Reverse((distance, vertex))) = heap.pop() {
            if distance > distances[vertex] {
                continue;
            }
            for &(neighbor, weight) in &reweighted[vertex] {
                let candidate = distance + weight;
                if candidate < distances[neighbor] {
                    distances[neighbor] = candidate;
                    heap.push(std::cmp::Reverse((candidate, neighbor)));
                }
            }
        }
        for target in 0..n {
            if distances[target] != i32::MAX {
                result[source][target] =
                    Some(distances[target] - potentials[source] + potentials[target]);
            }
        }
    }
    Ok((result, edges.len(), bellman_ford_relaxations, n))
}

fn bellman_ford_from_edges(
    edges: &[(usize, usize, i32)],
    n: usize,
    start: usize,
) -> Result<(Vec<i32>, Vec<Option<usize>>), NegativeCycle> {
    let mut dist = vec![i32::MAX; n];
    let mut prev = vec![None; n];
    dist[start] = 0;
    for _ in 0..n.saturating_sub(1) {
        let mut relaxed = false;
        for &(u, v, w) in edges {
            if dist[u] != i32::MAX && dist[u] + w < dist[v] {
                dist[v] = dist[u] + w;
                prev[v] = Some(u);
                relaxed = true;
            }
        }
        if !relaxed {
            break;
        }
    }
    for &(u, v, w) in edges {
        if dist[u] != i32::MAX && dist[u] + w < dist[v] {
            return Err(NegativeCycle);
        }
    }
    Ok((dist, prev))
}

unsafe fn read_unweighted_adjacency(rows: &[TyrusArray]) -> Vec<Vec<usize>> {
    let mut adjacency = Vec::with_capacity(rows.len());
    for row in rows {
        let mut neighbors = Vec::with_capacity(row.length);
        if !row.data.is_null() {
            let values =
                unsafe { std::slice::from_raw_parts(row.data.cast::<usize>(), row.length) };
            neighbors.extend_from_slice(values);
        }
        adjacency.push(neighbors);
    }
    adjacency
}

unsafe fn read_weighted_adjacency(
    rows: &[TyrusArray],
    tuple_stride: usize,
) -> Vec<Vec<(usize, i32)>> {
    let mut adjacency = Vec::with_capacity(rows.len());
    for row in rows {
        let mut neighbors = Vec::with_capacity(row.length);
        if !row.data.is_null() {
            for i in 0..row.length {
                let slot = unsafe { row.data.add(i * tuple_stride) };
                let to = unsafe { ptr::read_unaligned(slot.cast::<usize>()) };
                let weight = unsafe { ptr::read_unaligned(slot.add(8).cast::<i32>()) };
                neighbors.push((to, weight));
            }
        }
        adjacency.push(neighbors);
    }
    adjacency
}

unsafe fn read_weight_matrix(rows: &[TyrusArray]) -> Vec<Vec<Option<i32>>> {
    let mut matrix = Vec::with_capacity(rows.len());
    for row in rows {
        let mut values = Vec::with_capacity(row.length);
        if !row.data.is_null() {
            let slots =
                unsafe { std::slice::from_raw_parts(row.data.cast::<*const i32>(), row.length) };
            for &slot in slots {
                if slot.is_null() {
                    values.push(None);
                } else {
                    values.push(Some(unsafe { *slot }));
                }
            }
        }
        matrix.push(values);
    }
    matrix
}

fn graph_unweighted_adjacency(graph: &TyrusGraph) -> Vec<Vec<usize>> {
    graph
        .adjacency()
        .iter()
        .map(|neighbors| neighbors.iter().map(|edge| edge.to).collect())
        .collect()
}

fn graph_weighted_adjacency(graph: &TyrusGraph) -> Vec<Vec<(usize, i32)>> {
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

fn graph_weight_matrix(graph: &TyrusGraph) -> Vec<Vec<Option<i32>>> {
    let n = graph.node_count();
    let mut matrix = vec![vec![None; n]; n];
    for (vertex, row) in matrix.iter_mut().enumerate() {
        row[vertex] = Some(0);
    }
    for (u, neighbors) in graph.adjacency().iter().enumerate() {
        for edge in neighbors {
            matrix[u][edge.to] = Some(graph.weight_as_i32(&edge.weight));
        }
    }
    matrix
}

unsafe fn fill_usize_array(output: *mut TyrusArray, values: &[usize]) {
    let Some(output) = (unsafe { output.as_mut() }) else {
        return;
    };
    let data = if values.is_empty() {
        ptr::null_mut()
    } else {
        unsafe { malloc(std::mem::size_of_val(values)).cast::<u8>() }
    };
    if !values.is_empty() && data.is_null() {
        allocation_failed();
    }
    if !values.is_empty() {
        unsafe {
            ptr::copy_nonoverlapping(
                values.as_ptr().cast::<u8>(),
                data,
                std::mem::size_of_val(values),
            );
        }
    }
    *output = TyrusArray {
        data,
        length: values.len(),
        capacity: values.len(),
    };
}

unsafe fn fill_i32_array(output: *mut TyrusArray, values: &[i32]) {
    let Some(output) = (unsafe { output.as_mut() }) else {
        return;
    };
    let data = if values.is_empty() {
        ptr::null_mut()
    } else {
        unsafe { malloc(std::mem::size_of_val(values)).cast::<u8>() }
    };
    if !values.is_empty() && data.is_null() {
        allocation_failed();
    }
    if !values.is_empty() {
        unsafe {
            ptr::copy_nonoverlapping(
                values.as_ptr().cast::<u8>(),
                data,
                std::mem::size_of_val(values),
            );
        }
    }
    *output = TyrusArray {
        data,
        length: values.len(),
        capacity: values.len(),
    };
}

unsafe fn fill_optional_usize_array(output: *mut TyrusArray, values: &[Option<usize>]) {
    let Some(output) = (unsafe { output.as_mut() }) else {
        return;
    };
    let ptr_size = std::mem::size_of::<*const usize>();
    let data = if values.is_empty() {
        ptr::null_mut()
    } else {
        unsafe { malloc(values.len() * ptr_size).cast::<u8>() }
    };
    if !values.is_empty() && data.is_null() {
        allocation_failed();
    }
    let slots = if values.is_empty() {
        &mut []
    } else {
        unsafe { std::slice::from_raw_parts_mut(data.cast::<*mut usize>(), values.len()) }
    };
    for (index, value) in values.iter().enumerate() {
        match value {
            Some(present) => {
                let slot = unsafe { malloc(std::mem::size_of::<usize>()) }.cast::<usize>();
                if slot.is_null() {
                    allocation_failed();
                }
                unsafe { *slot = *present };
                slots[index] = slot;
            }
            None => slots[index] = ptr::null_mut(),
        }
    }
    *output = TyrusArray {
        data,
        length: values.len(),
        capacity: values.len(),
    };
}

unsafe fn fill_optional_matrix(output: *mut TyrusArray, matrix: &[Vec<Option<i32>>]) {
    let Some(output) = (unsafe { output.as_mut() }) else {
        return;
    };
    let row_count = matrix.len();
    let row_struct_size = std::mem::size_of::<TyrusArray>();
    let ptr_size = std::mem::size_of::<*const i32>();
    let rows_data = if row_count == 0 {
        ptr::null_mut()
    } else {
        unsafe { malloc(row_count * row_struct_size).cast::<u8>() }
    };
    if row_count > 0 && rows_data.is_null() {
        allocation_failed();
    }
    let rows = if row_count == 0 {
        &mut []
    } else {
        unsafe { std::slice::from_raw_parts_mut(rows_data.cast::<TyrusArray>(), row_count) }
    };
    for (row_index, row_values) in matrix.iter().enumerate() {
        let length = row_values.len();
        let data = if length == 0 {
            ptr::null_mut()
        } else {
            unsafe { malloc(length * ptr_size).cast::<u8>() }
        };
        if length > 0 && data.is_null() {
            allocation_failed();
        }
        let slots = if length == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(data.cast::<*mut i32>(), length) }
        };
        for (column, value) in row_values.iter().enumerate() {
            match value {
                Some(present) => {
                    let slot = unsafe { malloc(std::mem::size_of::<i32>()) }.cast::<i32>();
                    if slot.is_null() {
                        allocation_failed();
                    }
                    unsafe { *slot = *present };
                    slots[column] = slot;
                }
                None => slots[column] = ptr::null_mut(),
            }
        }
        rows[row_index] = TyrusArray {
            data,
            length,
            capacity: length,
        };
    }
    *output = TyrusArray {
        data: rows_data,
        length: row_count,
        capacity: row_count,
    };
}

unsafe fn optional_i32(value: Option<i32>) -> *mut i32 {
    unsafe {
        let Some(value) = value else {
            return ptr::null_mut();
        };
        let output = malloc(std::mem::size_of::<i32>()).cast::<i32>();
        if output.is_null() {
            allocation_failed();
        }
        *output = value;
        output
    }
}

unsafe fn fill_shortest_path_arrays(
    distances: *mut TyrusArray,
    predecessors: *mut TyrusArray,
    study: &ShortestPathStudy,
) {
    unsafe {
        fill_i32_array(distances, &study.distances);
        fill_optional_usize_array(predecessors, &study.predecessors);
    }
}

unsafe fn fill_dijkstra_info(output: *mut TyrusDijkstraInfo, study: DijkstraStudy) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let mut result = TyrusDijkstraInfo {
            distances: TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            },
            predecessors: TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            },
            relaxation_count: study.base.relaxation_count,
            processed_vertex_count: study.base.processed_vertex_count,
            examined_edge_count: study.base.examined_edge_count,
            heap_extraction_count: study.heap_extraction_count,
            accepted_relaxation_count: study.base.relaxation_count,
            finalized_vertex_count: study.finalized_vertex_count,
        };
        fill_shortest_path_arrays(&mut result.distances, &mut result.predecessors, &study.base);
        *output = result;
    }
}

unsafe fn fill_a_star_info(output: *mut TyrusAStarInfo, study: AStarStudy) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let mut result = TyrusAStarInfo {
            distances: TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            },
            predecessors: TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            },
            relaxation_count: study.base.relaxation_count,
            processed_vertex_count: study.base.processed_vertex_count,
            examined_edge_count: study.base.examined_edge_count,
            heuristic_evaluation_count: study.heuristic_evaluation_count,
            opened_vertex_count: study.opened_vertex_count,
            closed_vertex_count: study.closed_vertex_count,
            final_cost: optional_i32(study.final_cost),
        };
        fill_shortest_path_arrays(&mut result.distances, &mut result.predecessors, &study.base);
        *output = result;
    }
}

unsafe fn fill_bellman_ford_info(output: *mut TyrusBellmanFordInfo, study: BellmanFordStudy) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let negative_cycle_edge = study.negative_cycle_edge.map_or(ptr::null_mut(), |edge| {
            Box::into_raw(Box::new(TyrusWeightedEdge {
                from: edge.0,
                to: edge.1,
                weight: edge.2,
            }))
        });
        let mut result = TyrusBellmanFordInfo {
            distances: TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            },
            predecessors: TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            },
            relaxation_count: study.base.relaxation_count,
            processed_vertex_count: study.base.processed_vertex_count,
            examined_edge_count: study.base.examined_edge_count,
            pass_count: study.pass_count,
            negative_cycle_detected: study.negative_cycle_edge.is_some(),
            negative_cycle_edge,
        };
        fill_shortest_path_arrays(&mut result.distances, &mut result.predecessors, &study.base);
        *output = result;
    }
}

unsafe fn fill_spfa_info(output: *mut TyrusSpfaInfo, study: SpfaStudy) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let mut result = TyrusSpfaInfo {
            distances: TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            },
            predecessors: TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            },
            relaxation_count: study.base.relaxation_count,
            processed_vertex_count: study.base.processed_vertex_count,
            examined_edge_count: study.base.examined_edge_count,
            queue_push_count: study.queue_push_count,
            queue_pop_count: study.queue_pop_count,
            max_queue_size: study.max_queue_size,
        };
        fill_shortest_path_arrays(&mut result.distances, &mut result.predecessors, &study.base);
        *output = result;
    }
}

unsafe fn fill_johnson_info(
    output: *mut TyrusJohnsonInfo,
    study: (Vec<Vec<Option<i32>>>, usize, usize, usize),
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let mut distances = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_optional_matrix(&mut distances, &study.0);
        *output = TyrusJohnsonInfo {
            distances,
            reweighting_count: study.1,
            bellman_ford_relaxation_count: study.2,
            dijkstra_execution_count: study.3,
        };
    }
}

unsafe fn fill_floyd_warshall_info(
    output: *mut TyrusFloydWarshallInfo,
    study: (Vec<Vec<Option<i32>>>, usize, usize, bool),
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let mut distances = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_optional_matrix(&mut distances, &study.0);
        *output = TyrusFloydWarshallInfo {
            distances,
            k_pass_count: study.1,
            distance_update_count: study.2,
            negative_cycle_detected: study.3,
        };
    }
}

#[repr(C)]
pub struct PathResult {
    pub path: TyrusArray,
    pub distance: i32,
}

// ─── Unweighted traversal ABI ───

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_bfs(
    adj_data: *const TyrusArray,
    adj_length: usize,
    start: usize,
    output: *mut TyrusArray,
) {
    let rows = unsafe { std::slice::from_raw_parts(adj_data, adj_length) };
    let adjacency = unsafe { read_unweighted_adjacency(rows) };
    let order = bfs(&adjacency, start);
    unsafe { fill_usize_array(output, &order) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_bfs_graph(
    graph: *const TyrusGraph,
    start: usize,
    output: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.expect("tyrus runtime error: null Graph handle");
    let adjacency = graph_unweighted_adjacency(graph);
    let order = bfs(&adjacency, start);
    unsafe { fill_usize_array(output, &order) };
}

unsafe fn fill_traversal_info(
    output: *mut TyrusTraversalInfo,
    start: usize,
    result: (Vec<usize>, usize, usize, usize),
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let (order, examined_edges, max_frontier_size, max_depth) = result;
        let mut visit_order = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        fill_usize_array(&mut visit_order, &order);
        *output = TyrusTraversalInfo {
            start,
            visit_order,
            visited_count: order.len(),
            examined_edges,
            max_frontier_size,
            max_depth,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_bfs_info(
    adj_data: *const TyrusArray,
    adj_length: usize,
    start: usize,
    output: *mut TyrusTraversalInfo,
) {
    unsafe {
        let adjacency = read_unweighted_adjacency(std::slice::from_raw_parts(adj_data, adj_length));
        fill_traversal_info(output, start, bfs_info(&adjacency, start));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_bfs_info_graph(
    graph: *const TyrusGraph,
    start: usize,
    output: *mut TyrusTraversalInfo,
) {
    unsafe {
        let adjacency = graph_unweighted_adjacency(
            graph
                .as_ref()
                .expect("tyrus runtime error: null Graph handle"),
        );
        fill_traversal_info(output, start, bfs_info(&adjacency, start));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_dfs(
    adj_data: *const TyrusArray,
    adj_length: usize,
    start: usize,
    output: *mut TyrusArray,
) {
    let rows = unsafe { std::slice::from_raw_parts(adj_data, adj_length) };
    let adjacency = unsafe { read_unweighted_adjacency(rows) };
    let order = dfs(&adjacency, start);
    unsafe { fill_usize_array(output, &order) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_dfs_graph(
    graph: *const TyrusGraph,
    start: usize,
    output: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.expect("tyrus runtime error: null Graph handle");
    let adjacency = graph_unweighted_adjacency(graph);
    let order = dfs(&adjacency, start);
    unsafe { fill_usize_array(output, &order) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_dfs_info(
    adj_data: *const TyrusArray,
    adj_length: usize,
    start: usize,
    output: *mut TyrusTraversalInfo,
) {
    unsafe {
        let adjacency = read_unweighted_adjacency(std::slice::from_raw_parts(adj_data, adj_length));
        fill_traversal_info(output, start, dfs_info(&adjacency, start));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_dfs_info_graph(
    graph: *const TyrusGraph,
    start: usize,
    output: *mut TyrusTraversalInfo,
) {
    unsafe {
        let adjacency = graph_unweighted_adjacency(
            graph
                .as_ref()
                .expect("tyrus runtime error: null Graph handle"),
        );
        fill_traversal_info(output, start, dfs_info(&adjacency, start));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_iddfs(
    adj_data: *const TyrusArray,
    adj_length: usize,
    start: usize,
    max_depth: usize,
    output: *mut TyrusArray,
) {
    let rows = unsafe { std::slice::from_raw_parts(adj_data, adj_length) };
    let adjacency = unsafe { read_unweighted_adjacency(rows) };
    let order = iddfs(&adjacency, start, max_depth);
    unsafe { fill_usize_array(output, &order) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_iddfs_graph(
    graph: *const TyrusGraph,
    start: usize,
    max_depth: usize,
    output: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.expect("tyrus runtime error: null Graph handle");
    let adjacency = graph_unweighted_adjacency(graph);
    let order = iddfs(&adjacency, start, max_depth);
    unsafe { fill_usize_array(output, &order) };
}

unsafe fn fill_iddfs_info(
    output: *mut TyrusIddfsInfo,
    start: usize,
    result: (Vec<usize>, usize, usize, usize, Vec<usize>),
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let (order, examined_edges, max_frontier_size, max_depth, limits) = result;
        let (mut visit_order, mut depth_limits_tried) = (
            TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            },
            TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            },
        );
        fill_usize_array(&mut visit_order, &order);
        fill_usize_array(&mut depth_limits_tried, &limits);
        *output = TyrusIddfsInfo {
            start,
            visit_order,
            visited_count: order.len(),
            examined_edges,
            max_frontier_size,
            max_depth,
            depth_limits_tried,
            iterations: limits.len(),
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_iddfs_info(
    adj_data: *const TyrusArray,
    adj_length: usize,
    start: usize,
    max_depth: usize,
    output: *mut TyrusIddfsInfo,
) {
    unsafe {
        let adjacency = read_unweighted_adjacency(std::slice::from_raw_parts(adj_data, adj_length));
        fill_iddfs_info(output, start, iddfs_info(&adjacency, start, max_depth));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_iddfs_info_graph(
    graph: *const TyrusGraph,
    start: usize,
    max_depth: usize,
    output: *mut TyrusIddfsInfo,
) {
    unsafe {
        let adjacency = graph_unweighted_adjacency(
            graph
                .as_ref()
                .expect("tyrus runtime error: null Graph handle"),
        );
        fill_iddfs_info(output, start, iddfs_info(&adjacency, start, max_depth));
    }
}

// ─── Shortest-path ABI ───

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_dijkstra(
    adj_data: *const TyrusArray,
    adj_length: usize,
    tuple_stride: usize,
    start: usize,
    end: usize,
) -> *mut PathResult {
    let rows = unsafe { std::slice::from_raw_parts(adj_data, adj_length) };
    let adjacency = unsafe { read_weighted_adjacency(rows, tuple_stride) };
    path_result(dijkstra(&adjacency, start, end))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_dijkstra_graph(
    graph: *const TyrusGraph,
    start: usize,
    end: usize,
) -> *mut PathResult {
    let graph = unsafe { graph.as_ref() }.expect("tyrus runtime error: null Graph handle");
    let adjacency = graph_weighted_adjacency(graph);
    path_result(dijkstra(&adjacency, start, end))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_a_star(
    adj_data: *const TyrusArray,
    adj_length: usize,
    tuple_stride: usize,
    start: usize,
    end: usize,
    h_data: *const i32,
    h_length: usize,
) -> *mut PathResult {
    let rows = unsafe { std::slice::from_raw_parts(adj_data, adj_length) };
    let adjacency = unsafe { read_weighted_adjacency(rows, tuple_stride) };
    let h = unsafe { std::slice::from_raw_parts(h_data, h_length) };
    path_result(a_star(&adjacency, start, end, h))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_a_star_graph(
    graph: *const TyrusGraph,
    start: usize,
    end: usize,
    h_data: *const i32,
    h_length: usize,
) -> *mut PathResult {
    let graph = unsafe { graph.as_ref() }.expect("tyrus runtime error: null Graph handle");
    let adjacency = graph_weighted_adjacency(graph);
    let h = unsafe { std::slice::from_raw_parts(h_data, h_length) };
    path_result(a_star(&adjacency, start, end, h))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_dijkstra_info(
    adj_data: *const TyrusArray,
    adj_length: usize,
    tuple_stride: usize,
    start: usize,
    end: usize,
    output: *mut TyrusDijkstraInfo,
) {
    unsafe {
        let adjacency = read_weighted_adjacency(
            std::slice::from_raw_parts(adj_data, adj_length),
            tuple_stride,
        );
        fill_dijkstra_info(output, dijkstra_info(&adjacency, start, end));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_dijkstra_info_graph(
    graph: *const TyrusGraph,
    start: usize,
    end: usize,
    output: *mut TyrusDijkstraInfo,
) {
    unsafe {
        let adjacency = graph_weighted_adjacency(
            graph
                .as_ref()
                .expect("tyrus runtime error: null Graph handle"),
        );
        fill_dijkstra_info(output, dijkstra_info(&adjacency, start, end));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_a_star_info(
    adj_data: *const TyrusArray,
    adj_length: usize,
    tuple_stride: usize,
    start: usize,
    end: usize,
    h_data: *const i32,
    h_length: usize,
    output: *mut TyrusAStarInfo,
) {
    unsafe {
        let adjacency = read_weighted_adjacency(
            std::slice::from_raw_parts(adj_data, adj_length),
            tuple_stride,
        );
        let heuristic = std::slice::from_raw_parts(h_data, h_length);
        fill_a_star_info(output, a_star_info(&adjacency, start, end, heuristic));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_a_star_info_graph(
    graph: *const TyrusGraph,
    start: usize,
    end: usize,
    h_data: *const i32,
    h_length: usize,
    output: *mut TyrusAStarInfo,
) {
    unsafe {
        let adjacency = graph_weighted_adjacency(
            graph
                .as_ref()
                .expect("tyrus runtime error: null Graph handle"),
        );
        let heuristic = std::slice::from_raw_parts(h_data, h_length);
        fill_a_star_info(output, a_star_info(&adjacency, start, end, heuristic));
    }
}

fn path_result(found: Option<(Vec<usize>, i32)>) -> *mut PathResult {
    match found {
        Some((path, distance)) => {
            let mut result = Box::new(PathResult {
                path: TyrusArray {
                    data: ptr::null_mut(),
                    length: 0,
                    capacity: 0,
                },
                distance,
            });
            // Safety: `path` is a local vec we still own; fill the nested array
            // before the box is handed to the caller.
            unsafe { fill_usize_array(&mut result.path, &path) };
            Box::into_raw(result)
        }
        None => ptr::null_mut(),
    }
}

// ─── Negative-weight ABI ───

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_bellman_ford(
    adj_data: *const TyrusArray,
    adj_length: usize,
    tuple_stride: usize,
    start: usize,
    out_dists: *mut TyrusArray,
    out_prev: *mut TyrusArray,
) {
    let rows = unsafe { std::slice::from_raw_parts(adj_data, adj_length) };
    let adjacency = unsafe { read_weighted_adjacency(rows, tuple_stride) };
    match bellman_ford(&adjacency, start) {
        Ok((dists, prev)) => unsafe {
            fill_i32_array(out_dists, &dists);
            fill_optional_usize_array(out_prev, &prev);
        },
        Err(NegativeCycle) => {
            set_algorithm_error(
                "ERR_ALGORITHM_NEGATIVE_CYCLE",
                "negative-weight cycle detected",
            );
            unsafe {
                fill_i32_array(out_dists, &[]);
                fill_optional_usize_array(out_prev, &[]);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_bellman_ford_graph(
    graph: *const TyrusGraph,
    start: usize,
    out_dists: *mut TyrusArray,
    out_prev: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.expect("tyrus runtime error: null Graph handle");
    let adjacency = graph_weighted_adjacency(graph);
    match bellman_ford(&adjacency, start) {
        Ok((dists, prev)) => unsafe {
            fill_i32_array(out_dists, &dists);
            fill_optional_usize_array(out_prev, &prev);
        },
        Err(NegativeCycle) => {
            set_algorithm_error(
                "ERR_ALGORITHM_NEGATIVE_CYCLE",
                "negative-weight cycle detected",
            );
            unsafe {
                fill_i32_array(out_dists, &[]);
                fill_optional_usize_array(out_prev, &[]);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_spfa(
    adj_data: *const TyrusArray,
    adj_length: usize,
    tuple_stride: usize,
    start: usize,
    out_dists: *mut TyrusArray,
    out_prev: *mut TyrusArray,
) {
    let rows = unsafe { std::slice::from_raw_parts(adj_data, adj_length) };
    let adjacency = unsafe { read_weighted_adjacency(rows, tuple_stride) };
    match spfa(&adjacency, start) {
        Ok((dists, prev)) => unsafe {
            fill_i32_array(out_dists, &dists);
            fill_optional_usize_array(out_prev, &prev);
        },
        Err(NegativeCycle) => {
            set_algorithm_error(
                "ERR_ALGORITHM_NEGATIVE_CYCLE",
                "negative-weight cycle detected",
            );
            unsafe {
                fill_i32_array(out_dists, &[]);
                fill_optional_usize_array(out_prev, &[]);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_spfa_graph(
    graph: *const TyrusGraph,
    start: usize,
    out_dists: *mut TyrusArray,
    out_prev: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.expect("tyrus runtime error: null Graph handle");
    let adjacency = graph_weighted_adjacency(graph);
    match spfa(&adjacency, start) {
        Ok((dists, prev)) => unsafe {
            fill_i32_array(out_dists, &dists);
            fill_optional_usize_array(out_prev, &prev);
        },
        Err(NegativeCycle) => {
            set_algorithm_error(
                "ERR_ALGORITHM_NEGATIVE_CYCLE",
                "negative-weight cycle detected",
            );
            unsafe {
                fill_i32_array(out_dists, &[]);
                fill_optional_usize_array(out_prev, &[]);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_bellman_ford_info(
    adj_data: *const TyrusArray,
    adj_length: usize,
    tuple_stride: usize,
    start: usize,
    output: *mut TyrusBellmanFordInfo,
) {
    unsafe {
        let adjacency = read_weighted_adjacency(
            std::slice::from_raw_parts(adj_data, adj_length),
            tuple_stride,
        );
        fill_bellman_ford_info(output, bellman_ford_info(&adjacency, start));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_bellman_ford_info_graph(
    graph: *const TyrusGraph,
    start: usize,
    output: *mut TyrusBellmanFordInfo,
) {
    unsafe {
        let adjacency = graph_weighted_adjacency(
            graph
                .as_ref()
                .expect("tyrus runtime error: null Graph handle"),
        );
        fill_bellman_ford_info(output, bellman_ford_info(&adjacency, start));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_spfa_info(
    adj_data: *const TyrusArray,
    adj_length: usize,
    tuple_stride: usize,
    start: usize,
    output: *mut TyrusSpfaInfo,
) {
    unsafe {
        let adjacency = read_weighted_adjacency(
            std::slice::from_raw_parts(adj_data, adj_length),
            tuple_stride,
        );
        match spfa_info(&adjacency, start) {
            Ok(study) => fill_spfa_info(output, study),
            Err(NegativeCycle) => set_algorithm_error(
                "ERR_ALGORITHM_NEGATIVE_CYCLE",
                "negative-weight cycle detected",
            ),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_spfa_info_graph(
    graph: *const TyrusGraph,
    start: usize,
    output: *mut TyrusSpfaInfo,
) {
    unsafe {
        let adjacency = graph_weighted_adjacency(
            graph
                .as_ref()
                .expect("tyrus runtime error: null Graph handle"),
        );
        match spfa_info(&adjacency, start) {
            Ok(study) => fill_spfa_info(output, study),
            Err(NegativeCycle) => set_algorithm_error(
                "ERR_ALGORITHM_NEGATIVE_CYCLE",
                "negative-weight cycle detected",
            ),
        }
    }
}

// ─── All-pairs ABI ───

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_floyd_warshall(
    matrix_data: *const TyrusArray,
    matrix_length: usize,
    output: *mut TyrusArray,
) {
    let rows = unsafe { std::slice::from_raw_parts(matrix_data, matrix_length) };
    let matrix = unsafe { read_weight_matrix(rows) };
    let result = floyd_warshall(&matrix);
    unsafe { fill_optional_matrix(output, &result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_floyd_warshall_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.expect("tyrus runtime error: null Graph handle");
    let matrix = graph_weight_matrix(graph);
    let result = floyd_warshall(&matrix);
    unsafe { fill_optional_matrix(output, &result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_johnson(
    adj_data: *const TyrusArray,
    adj_length: usize,
    tuple_stride: usize,
    output: *mut TyrusArray,
) {
    let rows = unsafe { std::slice::from_raw_parts(adj_data, adj_length) };
    let adjacency = unsafe { read_weighted_adjacency(rows, tuple_stride) };
    match johnson(&adjacency) {
        Ok(result) => unsafe { fill_optional_matrix(output, &result) },
        Err(NegativeCycle) => {
            set_algorithm_error(
                "ERR_ALGORITHM_NEGATIVE_CYCLE",
                "negative-weight cycle detected",
            );
            unsafe { fill_optional_matrix(output, &[]) };
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_johnson_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusArray,
) {
    let graph = unsafe { graph.as_ref() }.expect("tyrus runtime error: null Graph handle");
    let adjacency = graph_weighted_adjacency(graph);
    match johnson(&adjacency) {
        Ok(result) => unsafe { fill_optional_matrix(output, &result) },
        Err(NegativeCycle) => {
            set_algorithm_error(
                "ERR_ALGORITHM_NEGATIVE_CYCLE",
                "negative-weight cycle detected",
            );
            unsafe { fill_optional_matrix(output, &[]) };
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_floyd_warshall_info(
    matrix_data: *const TyrusArray,
    matrix_length: usize,
    output: *mut TyrusFloydWarshallInfo,
) {
    unsafe {
        let matrix = read_weight_matrix(std::slice::from_raw_parts(matrix_data, matrix_length));
        fill_floyd_warshall_info(output, floyd_warshall_info(&matrix));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_floyd_warshall_info_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusFloydWarshallInfo,
) {
    unsafe {
        let matrix = graph_weight_matrix(
            graph
                .as_ref()
                .expect("tyrus runtime error: null Graph handle"),
        );
        fill_floyd_warshall_info(output, floyd_warshall_info(&matrix));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_johnson_info(
    adj_data: *const TyrusArray,
    adj_length: usize,
    tuple_stride: usize,
    output: *mut TyrusJohnsonInfo,
) {
    unsafe {
        let adjacency = read_weighted_adjacency(
            std::slice::from_raw_parts(adj_data, adj_length),
            tuple_stride,
        );
        match johnson_info(&adjacency) {
            Ok(study) => fill_johnson_info(output, study),
            Err(NegativeCycle) => set_algorithm_error(
                "ERR_ALGORITHM_NEGATIVE_CYCLE",
                "negative-weight cycle detected",
            ),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_johnson_info_graph(
    graph: *const TyrusGraph,
    output: *mut TyrusJohnsonInfo,
) {
    unsafe {
        let adjacency = graph_weighted_adjacency(
            graph
                .as_ref()
                .expect("tyrus runtime error: null Graph handle"),
        );
        match johnson_info(&adjacency) {
            Ok(study) => fill_johnson_info(output, study),
            Err(NegativeCycle) => set_algorithm_error(
                "ERR_ALGORITHM_NEGATIVE_CYCLE",
                "negative-weight cycle detected",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bfs_visits_in_width_order() {
        let adj = vec![vec![1, 2], vec![0, 3], vec![0], vec![1]];
        assert_eq!(bfs(&adj, 0), vec![0, 1, 2, 3]);
        assert_eq!(bfs(&adj, 3), vec![3, 1, 0, 2]);
    }

    #[test]
    fn dfs_visits_in_depth_order() {
        let adj = vec![vec![1, 2], vec![0, 3], vec![0], vec![1]];
        assert_eq!(dfs(&adj, 0), vec![0, 1, 3, 2]);
    }

    #[test]
    fn iddfs_respects_depth_limit() {
        let adj = vec![vec![1], vec![2], vec![3], vec![]];
        assert_eq!(iddfs(&adj, 0, 0), vec![0]);
        assert_eq!(iddfs(&adj, 0, 1), vec![0, 1]);
        assert_eq!(iddfs(&adj, 0, 2), vec![0, 1, 2]);
        assert_eq!(iddfs(&adj, 0, 3), vec![0, 1, 2, 3]);
    }

    #[test]
    fn dijkstra_finds_shortest_path() {
        let adj = vec![
            vec![(1, 4), (2, 2)],
            vec![(2, 1), (3, 5)],
            vec![(3, 8)],
            vec![],
        ];
        let (path, dist) = dijkstra(&adj, 0, 3).unwrap();
        assert_eq!(path, vec![0, 1, 3]);
        assert_eq!(dist, 9);
        assert!(dijkstra(&adj, 3, 0).is_none());
    }

    #[test]
    fn a_star_matches_dijkstra_with_zero_heuristic() {
        let adj = vec![
            vec![(1, 4), (2, 2)],
            vec![(2, 1), (3, 5)],
            vec![(3, 8)],
            vec![],
        ];
        let h = vec![0, 0, 0, 0];
        let (path, dist) = a_star(&adj, 0, 3, &h).unwrap();
        assert_eq!(path, vec![0, 1, 3]);
        assert_eq!(dist, 9);
    }

    #[test]
    fn bellman_ford_supports_negative_weights() {
        let adj = vec![vec![(1, 1)], vec![(2, -3)], vec![(0, 2)]];
        let (dists, prev) = bellman_ford(&adj, 0).unwrap();
        assert_eq!(dists, vec![0, 1, -2]);
        assert_eq!(prev, vec![None, Some(0), Some(1)]);
    }

    #[test]
    fn bellman_ford_detects_negative_cycle() {
        let adj = vec![vec![(1, 1)], vec![(2, -3)], vec![(0, -1)]];
        assert!(bellman_ford(&adj, 0).is_err());
    }

    #[test]
    fn spfa_matches_bellman_ford() {
        let adj = vec![vec![(1, 1)], vec![(2, -3)], vec![(0, 2)]];
        assert_eq!(spfa(&adj, 0).unwrap().0, vec![0, 1, -2]);
        let cycle = vec![vec![(1, 1)], vec![(2, -3)], vec![(0, -1)]];
        assert!(spfa(&cycle, 0).is_err());
    }

    #[test]
    fn floyd_warshall_all_pairs() {
        let matrix = vec![
            vec![Some(0), Some(5), None],
            vec![None, Some(0), Some(2)],
            vec![Some(3), None, Some(0)],
        ];
        let result = floyd_warshall(&matrix);
        assert_eq!(result[0][2], Some(7));
        assert_eq!(result[2][1], Some(8));
        assert_eq!(result[0][0], Some(0));
    }

    #[test]
    fn johnson_matches_floyd_warshall() {
        let adj = vec![vec![(1, 5), (2, 8)], vec![(2, 2)], vec![(0, 3)]];
        let result = johnson(&adj).unwrap();
        assert_eq!(result[0][2], Some(7));
        assert_eq!(result[2][1], Some(8));
        assert_eq!(result[1][0], Some(5));
    }

    #[test]
    fn johnson_detects_negative_cycle() {
        let adj = vec![vec![(1, 1)], vec![(2, -3)], vec![(0, -1)]];
        assert!(johnson(&adj).is_err());
    }
}
