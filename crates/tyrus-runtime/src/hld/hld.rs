use std::ffi::CString;
use std::ffi::c_void;
use std::ptr;

use crate::TyrusArray;
use crate::tyrus_panic;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

fn panic_out_of_bounds(what: &str) -> ! {
    let message =
        CString::new(format!("tyrus runtime error: HLD {what} is out of bounds")).unwrap();
    unsafe { tyrus_panic(message.as_ptr()) }
}

fn allocation_failed() -> ! {
    let message = CString::new("tyrus runtime error: HLD allocation failed").unwrap();
    unsafe { tyrus_panic(message.as_ptr()) }
}

pub struct TyrusHld {
    parent: Vec<usize>,
    head: Vec<usize>,
    pos: Vec<usize>,
}

fn build(adjacency: &[Vec<usize>], root: usize) -> TyrusHld {
    let n = adjacency.len();
    let mut size = vec![0usize; n];
    let mut parent = vec![0usize; n];
    let mut depth = vec![0usize; n];
    let mut heavy = vec![None; n];

    let mut stack = vec![(root, root, false)];
    while let Some((u, p, processed)) = stack.pop() {
        if processed {
            size[u] = 1;
            let mut max_child = 0;
            for &v in &adjacency[u] {
                if v != p {
                    size[u] += size[v];
                    if size[v] > max_child {
                        max_child = size[v];
                        heavy[u] = Some(v);
                    }
                }
            }
        } else {
            parent[u] = p;
            stack.push((u, p, true));
            for &v in &adjacency[u] {
                if v != p {
                    depth[v] = depth[u] + 1;
                    stack.push((v, u, false));
                }
            }
        }
    }

    let mut head = vec![0usize; n];
    let mut pos = vec![0usize; n];
    let mut next_pos = 0usize;
    let mut chain_stack = vec![(root, root)];
    while let Some((u, h)) = chain_stack.pop() {
        head[u] = h;
        pos[u] = next_pos;
        next_pos += 1;
        let heavy_child = heavy[u];
        for &v in &adjacency[u] {
            if v != parent[u] && Some(v) != heavy_child {
                chain_stack.push((v, v));
            }
        }
        if let Some(hv) = heavy_child {
            chain_stack.push((hv, h));
        }
    }

    TyrusHld { parent, head, pos }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_hld_new(
    adj_data: *const TyrusArray,
    adj_length: usize,
    root: usize,
) -> *mut TyrusHld {
    unsafe {
        let mut adjacency: Vec<Vec<usize>> = Vec::with_capacity(adj_length);
        if !adj_data.is_null() {
            let rows = std::slice::from_raw_parts(adj_data, adj_length);
            for row in rows {
                let mut neighbors = Vec::with_capacity(row.length);
                if !row.data.is_null() {
                    let values = std::slice::from_raw_parts(row.data.cast::<usize>(), row.length);
                    neighbors.extend_from_slice(values);
                }
                adjacency.push(neighbors);
            }
        }
        if root >= adj_length {
            panic_out_of_bounds("root");
        }
        Box::into_raw(Box::new(build(&adjacency, root)))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_hld_query_path(
    hld: *const TyrusHld,
    u: usize,
    v: usize,
    pair_stride: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let Some(hld) = hld.as_ref() else {
            *output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        };
        let n = hld.pos.len();
        if u >= n {
            panic_out_of_bounds("u");
        }
        if v >= n {
            panic_out_of_bounds("v");
        }

        let mut ranges: Vec<(usize, usize)> = Vec::new();
        let mut a = u;
        let mut b = v;
        while hld.head[a] != hld.head[b] {
            if hld.pos[a] < hld.pos[b] {
                std::mem::swap(&mut a, &mut b);
            }
            let chain_head = hld.head[a];
            ranges.push((hld.pos[chain_head], hld.pos[a]));
            a = hld.parent[chain_head];
        }
        if hld.pos[a] < hld.pos[b] {
            ranges.push((hld.pos[a], hld.pos[b]));
        } else {
            ranges.push((hld.pos[b], hld.pos[a]));
        }

        let bytes = ranges
            .len()
            .checked_mul(pair_stride)
            .unwrap_or_else(|| allocation_failed());
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes).cast::<u8>()
        };
        if bytes != 0 && data.is_null() {
            allocation_failed();
        }

        let int_size = std::mem::size_of::<usize>();
        for (i, (l, r)) in ranges.iter().enumerate() {
            let slot = data.add(i * pair_stride);
            ptr::copy_nonoverlapping(l.to_le_bytes().as_ptr(), slot, int_size);
            ptr::copy_nonoverlapping(r.to_le_bytes().as_ptr(), slot.add(int_size), int_size);
        }

        *output = TyrusArray {
            data,
            length: ranges.len(),
            capacity: ranges.len(),
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_hld_position(hld: *const TyrusHld, v: usize) -> usize {
    unsafe {
        let hld = &*hld;
        if v >= hld.pos.len() {
            panic_out_of_bounds("v");
        }
        hld.pos[v]
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_hld_head(hld: *const TyrusHld, v: usize) -> usize {
    unsafe {
        let hld = &*hld;
        if v >= hld.pos.len() {
            panic_out_of_bounds("v");
        }
        hld.head[v]
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_hld_free(hld: *mut TyrusHld) {
    unsafe {
        if !hld.is_null() {
            drop(Box::from_raw(hld));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_path(n: usize) -> Vec<Vec<usize>> {
        (0..n)
            .map(|i| {
                if i == 0 {
                    vec![1]
                } else if i == n - 1 {
                    vec![n - 2]
                } else {
                    vec![i - 1, i + 1]
                }
            })
            .collect()
    }

    fn path_positions(hld: &TyrusHld, mut u: usize, mut v: usize) -> Vec<usize> {
        let mut path = Vec::new();
        while hld.pos[u] != hld.pos[v] {
            if hld.pos[u] < hld.pos[v] {
                std::mem::swap(&mut u, &mut v);
            }
            path.push(hld.pos[u]);
            u = hld.parent[u];
        }
        path.push(hld.pos[u]);
        path.sort_unstable();
        path
    }

    fn ranges_cover(ranges: &[(usize, usize)], expected: &[usize]) -> bool {
        let mut covered: Vec<usize> = ranges.iter().flat_map(|&(l, r)| l..=r).collect();
        covered.sort_unstable();
        covered == expected
    }

    #[test]
    fn path_tree_decomposition() {
        let tree = build_path(6);
        let hld = build(&tree, 0);
        assert_eq!(hld.pos, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(hld.head, vec![0, 0, 0, 0, 0, 0]);
        assert_eq!(hld.parent, vec![0, 0, 1, 2, 3, 4]);
    }

    #[test]
    fn star_tree_decomposition() {
        let tree = vec![vec![1, 2, 3], vec![0], vec![0], vec![0]];
        let hld = build(&tree, 0);
        assert_eq!(hld.head[0], 0);
        assert_eq!(hld.head[1], 0);
        assert_eq!(hld.head[2], 2);
        assert_eq!(hld.head[3], 3);
        assert_eq!(hld.pos[1], hld.pos[0] + 1);
    }

    #[test]
    fn query_path_matches_naive_on_random_trees() {
        let mut seed = 0x243F6A8885A308D3u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for n in 1..30usize {
            for _ in 0..50 {
                let mut tree: Vec<Vec<usize>> = vec![Vec::new(); n];
                for v in 1..n {
                    let parent = (next() as usize) % v;
                    tree[parent].push(v);
                    tree[v].push(parent);
                }
                let hld = build(&tree, 0);
                for _ in 0..50 {
                    let u = (next() as usize) % n;
                    let v = (next() as usize) % n;
                    let ranges = query_path(&hld, u, v);
                    let expected = path_positions(&hld, u, v);
                    assert!(
                        ranges_cover(&ranges, &expected),
                        "n={n} u={u} v={v} ranges={ranges:?} expected={expected:?}"
                    );
                }
            }
        }
    }

    fn query_path(hld: &TyrusHld, mut u: usize, mut v: usize) -> Vec<(usize, usize)> {
        let mut ranges = Vec::new();
        while hld.head[u] != hld.head[v] {
            if hld.pos[u] < hld.pos[v] {
                std::mem::swap(&mut u, &mut v);
            }
            let chain_head = hld.head[u];
            ranges.push((hld.pos[chain_head], hld.pos[u]));
            u = hld.parent[chain_head];
        }
        if hld.pos[u] < hld.pos[v] {
            ranges.push((hld.pos[u], hld.pos[v]));
        } else {
            ranges.push((hld.pos[v], hld.pos[u]));
        }
        ranges
    }
}
