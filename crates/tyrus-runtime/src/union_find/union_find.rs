use crate::TyrusArray;
use crate::tyrus_panic;
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut std::ffi::c_void;
}

fn index_out_of_bounds() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: UnionFind index is out of bounds".as_ptr()) }
}

struct UfNode {
    parent: usize,
    rank: u32,
}

pub struct TyrusUnionFind {
    nodes: Vec<UfNode>,
    set_count: usize,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_union_find_new(size: usize) -> *mut TyrusUnionFind {
    let nodes: Vec<UfNode> = (0..size).map(|i| UfNode { parent: i, rank: 0 }).collect();
    let uf = Box::new(TyrusUnionFind {
        nodes,
        set_count: size,
    });
    Box::into_raw(uf)
}

unsafe fn find_root(uf: &mut TyrusUnionFind, mut x: usize) -> usize {
    if x >= uf.nodes.len() {
        index_out_of_bounds();
    }
    let mut root = x;
    while uf.nodes[root].parent != root {
        root = uf.nodes[root].parent;
    }
    while uf.nodes[x].parent != x {
        let next = uf.nodes[x].parent;
        uf.nodes[x].parent = root;
        x = next;
    }
    root
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_union_find_find(uf: *mut TyrusUnionFind, x: usize) -> usize {
    unsafe {
        let uf = &mut *uf;
        find_root(uf, x)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_union_find_union(
    uf: *mut TyrusUnionFind,
    x: usize,
    y: usize,
) -> bool {
    unsafe {
        let uf = &mut *uf;
        let rx = find_root(uf, x);
        let ry = find_root(uf, y);
        if rx == ry {
            return false;
        }
        match uf.nodes[rx].rank.cmp(&uf.nodes[ry].rank) {
            std::cmp::Ordering::Less => uf.nodes[rx].parent = ry,
            std::cmp::Ordering::Greater => uf.nodes[ry].parent = rx,
            std::cmp::Ordering::Equal => {
                uf.nodes[ry].parent = rx;
                uf.nodes[rx].rank += 1;
            }
        }
        uf.set_count -= 1;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_union_find_connected(
    uf: *mut TyrusUnionFind,
    x: usize,
    y: usize,
) -> bool {
    unsafe {
        let uf = &mut *uf;
        find_root(uf, x) == find_root(uf, y)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_union_find_set_count(uf: *const TyrusUnionFind) -> usize {
    unsafe { (*uf).set_count }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_union_find_values(
    uf: *const TyrusUnionFind,
    element_size: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let Some(uf) = uf.as_ref() else {
            *output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        };
        let bytes = uf
            .nodes
            .len()
            .checked_mul(element_size)
            .unwrap_or_else(|| index_out_of_bounds());
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes).cast::<u8>()
        };
        if bytes != 0 && data.is_null() {
            index_out_of_bounds();
        }
        for index in 0..uf.nodes.len() {
            let slot = data.add(index * element_size);
            ptr::write_bytes(slot, 0, element_size);
            ptr::copy_nonoverlapping(
                (&index as *const usize).cast::<u8>(),
                slot,
                element_size.min(std::mem::size_of::<usize>()),
            );
        }
        *output = TyrusArray {
            data,
            length: uf.nodes.len(),
            capacity: uf.nodes.len(),
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_union_find_free(uf: *mut TyrusUnionFind) {
    unsafe {
        if !uf.is_null() {
            drop(Box::from_raw(uf));
        }
    }
}
