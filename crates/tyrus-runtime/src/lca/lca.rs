use std::ffi::CString;

use crate::TyrusArray;
use crate::tyrus_panic;

fn panic_out_of_bounds(what: &str) -> ! {
    let message =
        CString::new(format!("tyrus runtime error: LCA {what} is out of bounds")).unwrap();
    unsafe { tyrus_panic(message.as_ptr()) }
}

pub struct TyrusLca {
    up: Vec<Vec<Option<usize>>>,
    depth: Vec<usize>,
}

fn build(adjacency: &[Vec<usize>], root: usize) -> TyrusLca {
    let n = adjacency.len();
    let levels = if n == 0 {
        0
    } else {
        (usize::BITS - (n as u32).leading_zeros()) as usize
    };
    let mut up = vec![vec![None; n]; levels];
    let mut depth = vec![0usize; n];

    let mut stack = vec![(root, None)];
    while let Some((u, parent)) = stack.pop() {
        up[0][u] = parent;
        for &v in &adjacency[u] {
            if Some(v) != parent {
                depth[v] = depth[u] + 1;
                stack.push((v, Some(u)));
            }
        }
    }
    for k in 1..levels {
        for v in 0..n {
            if let Some(ancestor) = up[k - 1][v] {
                up[k][v] = up[k - 1][ancestor];
            }
        }
    }
    TyrusLca { up, depth }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_lca_new(
    adj_data: *const TyrusArray,
    adj_length: usize,
    root: usize,
) -> *mut TyrusLca {
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
pub unsafe extern "C" fn tyrus_lca_query(lca: *const TyrusLca, u: usize, v: usize) -> usize {
    unsafe {
        let lca = &*lca;
        query(lca, u, v)
    }
}

fn query(lca: &TyrusLca, u: usize, v: usize) -> usize {
    let n = lca.depth.len();
    if u >= n {
        panic_out_of_bounds("u");
    }
    if v >= n {
        panic_out_of_bounds("v");
    }
    if lca.up.is_empty() {
        return 0;
    }
    let (mut u, mut v) = (u, v);
    if lca.depth[u] < lca.depth[v] {
        std::mem::swap(&mut u, &mut v);
    }
    let mut diff = lca.depth[u] - lca.depth[v];
    let mut k = 0usize;
    while diff > 0 {
        if diff & 1 == 1 {
            u = lca.up[k][u].unwrap();
        }
        diff >>= 1;
        k += 1;
    }
    if u == v {
        return u;
    }
    for k in (0..lca.up.len()).rev() {
        if lca.up[k][u] != lca.up[k][v] {
            u = lca.up[k][u].unwrap();
            v = lca.up[k][v].unwrap();
        }
    }
    lca.up[0][u].unwrap_or(u)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_lca_free(lca: *mut TyrusLca) {
    unsafe {
        if !lca.is_null() {
            drop(Box::from_raw(lca));
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

    #[test]
    fn path_tree_lca() {
        let tree = build_path(6);
        let lca = build(&tree, 0);
        assert_eq!(lca.depth, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(query(&lca, 2, 5), 2);
        assert_eq!(query(&lca, 5, 2), 2);
        assert_eq!(query(&lca, 3, 4), 3);
        assert_eq!(query(&lca, 1, 1), 1);
        assert_eq!(query(&lca, 0, 5), 0);
    }

    #[test]
    fn balanced_tree_lca() {
        let tree = vec![
            vec![1, 2],
            vec![0, 3, 4],
            vec![0, 5, 6],
            vec![1],
            vec![1],
            vec![2],
            vec![2],
        ];
        let lca = build(&tree, 0);
        assert_eq!(query(&lca, 3, 4), 1);
        assert_eq!(query(&lca, 5, 6), 2);
        assert_eq!(query(&lca, 3, 6), 0);
        assert_eq!(query(&lca, 4, 5), 0);
        assert_eq!(query(&lca, 1, 2), 0);
    }

    #[test]
    fn root_is_its_own_lca() {
        let tree = vec![vec![1], vec![0, 2], vec![1]];
        let lca = build(&tree, 0);
        assert_eq!(query(&lca, 0, 2), 0);
        assert_eq!(query(&lca, 0, 0), 0);
    }

    #[test]
    fn matches_naive_on_random_trees() {
        let mut seed = 0x9E3779B97F4A7C15u64;
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
                let lca = build(&tree, 0);
                for _ in 0..50 {
                    let u = (next() as usize) % n;
                    let v = (next() as usize) % n;
                    assert_eq!(lca.query_naive(u, v), query(&lca, u, v), "u={u} v={v}");
                }
            }
        }
    }

    impl TyrusLca {
        fn query_naive(&self, u: usize, v: usize) -> usize {
            let (mut a, mut b) = (u, v);
            while self.depth[a] > self.depth[b] {
                a = self.up[0][a].unwrap();
            }
            while self.depth[b] > self.depth[a] {
                b = self.up[0][b].unwrap();
            }
            while a != b {
                a = self.up[0][a].unwrap();
                b = self.up[0][b].unwrap();
            }
            a
        }
    }
}
