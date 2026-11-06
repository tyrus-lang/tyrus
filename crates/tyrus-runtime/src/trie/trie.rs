use std::collections::HashMap;
use tyrus_string::TyrusString;

pub struct TyrusTrie {
    root: TrieNode,
    len: usize,
}

struct TrieNode {
    children: HashMap<u8, TrieNode>,
    is_end: bool,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            is_end: false,
        }
    }
}

fn insert_rec(node: &mut TrieNode, bytes: &[u8], inserted: &mut bool) {
    let mut node = node;
    for &byte in bytes {
        node = node.children.entry(byte).or_insert_with(TrieNode::new);
    }
    if !node.is_end {
        node.is_end = true;
        *inserted = true;
    }
}

fn find_node<'a>(node: &'a TrieNode, bytes: &[u8]) -> Option<&'a TrieNode> {
    let mut node = node;
    for &byte in bytes {
        node = node.children.get(&byte)?;
    }
    Some(node)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_trie_new() -> *mut TyrusTrie {
    Box::into_raw(Box::new(TyrusTrie {
        root: TrieNode::new(),
        len: 0,
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_trie_insert(trie: *mut TyrusTrie, word: TyrusString) {
    unsafe {
        let trie = &mut *trie;
        let bytes = word.as_str().as_bytes().to_vec();
        let mut inserted = false;
        insert_rec(&mut trie.root, &bytes, &mut inserted);
        if inserted {
            trie.len += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_trie_search(trie: *const TyrusTrie, word: TyrusString) -> bool {
    unsafe {
        let trie = &*trie;
        let bytes = word.as_str().as_bytes().to_vec();
        find_node(&trie.root, &bytes).is_some_and(|node| node.is_end)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_trie_starts_with(
    trie: *const TyrusTrie,
    prefix: TyrusString,
) -> bool {
    unsafe {
        let trie = &*trie;
        let bytes = prefix.as_str().as_bytes().to_vec();
        find_node(&trie.root, &bytes).is_some()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_trie_len(trie: *const TyrusTrie) -> usize {
    unsafe { (*trie).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_trie_is_empty(trie: *const TyrusTrie) -> bool {
    unsafe { (*trie).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_trie_clear(trie: *mut TyrusTrie) {
    unsafe {
        let trie = &mut *trie;
        trie.root.children.clear();
        trie.root.is_end = false;
        trie.len = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_trie_free(trie: *mut TyrusTrie) {
    unsafe {
        if !trie.is_null() {
            drop(Box::from_raw(trie));
        }
    }
}
