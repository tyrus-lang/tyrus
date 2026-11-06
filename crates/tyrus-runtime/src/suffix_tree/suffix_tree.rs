use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::ptr;
use std::slice;

use crate::TyrusArray;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

struct Node {
    start: usize,
    end: usize,
    link: isize,
    next: HashMap<u8, usize>,
}

fn edge_length(nodes: &[Node], node: usize, cur_pos: usize) -> usize {
    if nodes[node].end == usize::MAX {
        cur_pos + 1 - nodes[node].start
    } else {
        nodes[node].end + 1 - nodes[node].start
    }
}

fn build_suffix_tree(bytes: &[u8]) -> Vec<Node> {
    let mut nodes = vec![Node {
        start: 0,
        end: 0,
        link: -1,
        next: HashMap::new(),
    }];

    let mut active_node = 0usize;
    let mut active_edge = 0usize;
    let mut active_len = 0usize;
    let mut remainder = 0usize;

    for pos in 0..bytes.len() {
        let mut last_new_node: isize = -1;
        remainder += 1;

        while remainder > 0 {
            if active_len == 0 {
                active_edge = pos;
            }
            let edge_char = bytes[active_edge];

            match nodes[active_node].next.get(&edge_char) {
                None => {
                    let leaf = nodes.len();
                    nodes.push(Node {
                        start: pos,
                        end: usize::MAX,
                        link: -1,
                        next: HashMap::new(),
                    });
                    nodes[active_node].next.insert(edge_char, leaf);
                    if last_new_node != -1 {
                        nodes[last_new_node as usize].link = active_node as isize;
                    }
                    last_new_node = -1;
                }
                Some(&next) => {
                    let edge_len = edge_length(&nodes, next, pos);
                    if active_len >= edge_len {
                        active_edge += edge_len;
                        active_len -= edge_len;
                        active_node = next;
                        continue;
                    }
                    if bytes[nodes[next].start + active_len] == bytes[pos] {
                        if last_new_node != -1 {
                            nodes[last_new_node as usize].link = active_node as isize;
                        }
                        active_len += 1;
                        break;
                    }
                    let split = nodes.len();
                    nodes.push(Node {
                        start: nodes[next].start,
                        end: nodes[next].start + active_len - 1,
                        link: -1,
                        next: HashMap::new(),
                    });
                    let leaf = nodes.len();
                    nodes.push(Node {
                        start: pos,
                        end: usize::MAX,
                        link: -1,
                        next: HashMap::new(),
                    });

                    nodes[next].start += active_len;
                    let after_split = bytes[nodes[next].start];
                    nodes[split].next.insert(after_split, next);
                    nodes[split].next.insert(bytes[pos], leaf);
                    nodes[active_node].next.insert(edge_char, split);

                    if last_new_node != -1 {
                        nodes[last_new_node as usize].link = split as isize;
                    }
                    last_new_node = split as isize;
                }
            }

            remainder -= 1;

            if active_node == 0 {
                if active_len > 0 {
                    active_len -= 1;
                    active_edge = pos - remainder + 1;
                }
            } else if nodes[active_node].link >= 0 {
                active_node = nodes[active_node].link as usize;
            } else {
                active_node = 0;
            }
        }
    }

    nodes
}

fn collect_suffix_array(nodes: &[Node], text_len: usize) -> Vec<usize> {
    fn dfs(nodes: &[Node], text_len: usize, node: usize, depth: usize, out: &mut Vec<usize>) {
        if nodes[node].next.is_empty() {
            out.push(text_len - depth);
            return;
        }
        let mut children: Vec<(u8, usize)> = nodes[node]
            .next
            .iter()
            .map(|(&byte, &child)| (byte, child))
            .collect();
        children.sort_unstable_by_key(|&(byte, _)| byte);
        for (_, child) in children {
            let edge_len = edge_length(nodes, child, text_len - 1);
            dfs(nodes, text_len, child, depth + edge_len, out);
        }
    }
    let mut result = Vec::with_capacity(text_len);
    dfs(nodes, text_len, 0, 0, &mut result);
    result
}

fn pick_sentinel(text: &[u8]) -> Option<u8> {
    (0u8..=255).find(|&byte| !text.contains(&byte))
}

fn ukkonen_suffix_array(text: &[u8]) -> Vec<usize> {
    let Some(sentinel) = pick_sentinel(text) else {
        let mut suffixes: Vec<usize> = (0..text.len()).collect();
        suffixes.sort_unstable_by(|&i, &j| text[i..].cmp(&text[j..]));
        return suffixes;
    };
    let mut bytes = text.to_vec();
    bytes.push(sentinel);
    let extended_len = bytes.len();
    let nodes = build_suffix_tree(&bytes);
    let mut suffixes = collect_suffix_array(&nodes, extended_len);
    if !suffixes.is_empty() {
        suffixes.remove(0);
    }
    suffixes
}

pub struct TyrusSuffixTree {
    suffix_array: Vec<usize>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_suffix_tree_new(
    data: *const c_char,
    length: usize,
) -> *mut TyrusSuffixTree {
    unsafe {
        if data.is_null() || length == 0 {
            return Box::into_raw(Box::new(TyrusSuffixTree {
                suffix_array: Vec::new(),
            }));
        }
        let bytes = slice::from_raw_parts(data.cast::<u8>(), length).to_vec();
        let suffix_array = ukkonen_suffix_array(&bytes);
        Box::into_raw(Box::new(TyrusSuffixTree { suffix_array }))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_suffix_tree_get_suffix_array(
    tree: *const TyrusSuffixTree,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let tree = &*tree;
        let length = tree.suffix_array.len();
        if length == 0 {
            *output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        }
        let allocation = malloc(length * std::mem::size_of::<usize>()).cast::<usize>();
        if allocation.is_null() {
            *output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        }
        ptr::copy_nonoverlapping(tree.suffix_array.as_ptr(), allocation, length);
        *output = TyrusArray {
            data: allocation.cast::<u8>(),
            length,
            capacity: length,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_suffix_tree_free(tree: *mut TyrusSuffixTree) {
    unsafe {
        if !tree.is_null() {
            drop(Box::from_raw(tree));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive(text: &[u8]) -> Vec<usize> {
        let mut suffixes: Vec<usize> = (0..text.len()).collect();
        suffixes.sort_unstable_by(|&i, &j| text[i..].cmp(&text[j..]));
        suffixes
    }

    #[test]
    fn matches_naive_on_random_strings() {
        let mut seed = 0x9E3779B97F4A7C15u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let alphabet: &[u8] = b"ababcxyz";
        for length in 0..40usize {
            for _ in 0..200 {
                let text: Vec<u8> = (0..length)
                    .map(|_| alphabet[(next() as usize) % alphabet.len()])
                    .collect();
                assert_eq!(ukkonen_suffix_array(&text), naive(&text), "text: {text:?}");
            }
        }
    }

    #[test]
    fn banana() {
        assert_eq!(ukkonen_suffix_array(b"banana"), vec![5, 3, 1, 0, 4, 2]);
    }

    #[test]
    fn empty_and_single() {
        assert_eq!(ukkonen_suffix_array(b""), Vec::<usize>::new());
        assert_eq!(ukkonen_suffix_array(b"a"), vec![0]);
        assert_eq!(ukkonen_suffix_array(b"aa"), vec![1, 0]);
    }
}
