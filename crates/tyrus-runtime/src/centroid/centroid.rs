use std::ffi::CString;

use crate::TyrusArray;
use crate::tyrus_panic;

fn panic_out_of_bounds(what: &str) -> ! {
    let message = CString::new(format!(
        "tyrus runtime error: CentroidTree {what} is out of bounds"
    ))
    .unwrap();
    unsafe { tyrus_panic(message.as_ptr()) }
}

pub struct TyrusCentroidTree {
    parent: Vec<Option<usize>>,
    depth: Vec<usize>,
}

fn build(adjacency: &[Vec<usize>]) -> TyrusCentroidTree {
    let n = adjacency.len();
    let mut size = vec![0usize; n];
    let mut removed = vec![false; n];
    let mut parent = vec![None; n];
    let mut depth = vec![0usize; n];

    let mut stack = vec![(0usize, None::<usize>)];
    while let Some((start, parent_centroid)) = stack.pop() {
        if removed[start] {
            continue;
        }

        let mut dfs_stack = vec![(start, None::<usize>, false)];
        while let Some((u, p, processed)) = dfs_stack.pop() {
            if processed {
                size[u] = 1;
                for &v in &adjacency[u] {
                    if !removed[v] && Some(v) != p {
                        size[u] += size[v];
                    }
                }
            } else {
                dfs_stack.push((u, p, true));
                for &v in &adjacency[u] {
                    if !removed[v] && Some(v) != p {
                        dfs_stack.push((v, Some(u), false));
                    }
                }
            }
        }

        let total = size[start];
        let mut centroid = start;
        let mut p = None;
        loop {
            let mut next = None;
            for &v in &adjacency[centroid] {
                if !removed[v] && Some(v) != p && size[v] > total / 2 {
                    next = Some(v);
                    break;
                }
            }
            match next {
                Some(v) => {
                    p = Some(centroid);
                    centroid = v;
                }
                None => break,
            }
        }

        removed[centroid] = true;
        parent[centroid] = parent_centroid;
        if let Some(pc) = parent_centroid {
            depth[centroid] = depth[pc] + 1;
        }

        for &v in &adjacency[centroid] {
            if !removed[v] {
                stack.push((v, Some(centroid)));
            }
        }
    }

    TyrusCentroidTree { parent, depth }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_centroid_new(
    adj_data: *const TyrusArray,
    adj_length: usize,
) -> *mut TyrusCentroidTree {
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
        Box::into_raw(Box::new(build(&adjacency)))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_centroid_parent_of(
    ct: *const TyrusCentroidTree,
    v: usize,
    output: *mut usize,
) -> bool {
    unsafe {
        let ct = &*ct;
        if v >= ct.parent.len() {
            panic_out_of_bounds("v");
        }
        match ct.parent[v] {
            Some(parent) => {
                if !output.is_null() {
                    *output = parent;
                }
                true
            }
            None => false,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_centroid_depth(ct: *const TyrusCentroidTree, v: usize) -> usize {
    unsafe {
        let ct = &*ct;
        if v >= ct.depth.len() {
            panic_out_of_bounds("v");
        }
        ct.depth[v]
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_centroid_free(ct: *mut TyrusCentroidTree) {
    unsafe {
        if !ct.is_null() {
            drop(Box::from_raw(ct));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn centroid_of(tree: &[Vec<usize>]) -> TyrusCentroidTree {
        build(tree)
    }

    fn is_valid_centroid_decomposition(tree: &[Vec<usize>], ct: &TyrusCentroidTree) -> bool {
        let n = tree.len();
        if ct.parent.len() != n || ct.depth.len() != n {
            return false;
        }
        let mut roots = 0;
        for v in 0..n {
            match ct.parent[v] {
                None => roots += 1,
                Some(p) => {
                    if p >= n || ct.depth[v] != ct.depth[p] + 1 {
                        return false;
                    }
                }
            }
        }
        roots == 1
    }

    #[test]
    fn path_tree_decomposition() {
        let tree = vec![
            vec![1],
            vec![0, 2],
            vec![1, 3],
            vec![2, 4],
            vec![3, 5],
            vec![4],
        ];
        let ct = centroid_of(&tree);
        assert_eq!(ct.parent[2], None);
        assert_eq!(ct.depth[2], 0);
        assert_eq!(ct.parent[1], Some(2));
        assert_eq!(ct.depth[1], 1);
        assert_eq!(ct.parent[4], Some(2));
        assert_eq!(ct.depth[4], 1);
        assert_eq!(ct.parent[0], Some(1));
        assert_eq!(ct.parent[3], Some(4));
        assert_eq!(ct.parent[5], Some(4));
        assert_eq!(ct.depth[3], 2);
        assert!(is_valid_centroid_decomposition(&tree, &ct));
    }

    #[test]
    fn star_tree_decomposition() {
        let tree = vec![vec![1, 2, 3], vec![0], vec![0], vec![0]];
        let ct = centroid_of(&tree);
        assert_eq!(ct.parent[0], None);
        assert_eq!(ct.parent[1], Some(0));
        assert_eq!(ct.parent[2], Some(0));
        assert_eq!(ct.parent[3], Some(0));
        assert_eq!(ct.depth[0], 0);
        assert_eq!(ct.depth[1], 1);
        assert!(is_valid_centroid_decomposition(&tree, &ct));
    }

    #[test]
    fn single_vertex_tree() {
        let tree = vec![Vec::<usize>::new()];
        let ct = centroid_of(&tree);
        assert_eq!(ct.parent, vec![None]);
        assert_eq!(ct.depth, vec![0]);
        assert!(is_valid_centroid_decomposition(&tree, &ct));
    }

    #[test]
    fn decomposition_is_valid_on_random_trees() {
        let mut seed = 0x9E3779B97F4A7C15u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for n in 1..40usize {
            for _ in 0..100 {
                let mut tree: Vec<Vec<usize>> = vec![Vec::new(); n];
                for v in 1..n {
                    let parent = (next() as usize) % v;
                    tree[parent].push(v);
                    tree[v].push(parent);
                }
                let ct = centroid_of(&tree);
                assert!(
                    is_valid_centroid_decomposition(&tree, &ct),
                    "n={n} tree={tree:?} parent={:?} depth={:?}",
                    ct.parent,
                    ct.depth
                );
            }
        }
    }
}
