use super::*;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

const MAX_LEVEL: usize = 16;

const NONE: usize = usize::MAX;

static SEED: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

struct SkipNode {
    key: Vec<u8>,
    next: Vec<usize>,
}

pub struct TyrusSkipList {
    nodes: Vec<SkipNode>,
    free: Vec<usize>,
    head: usize,
    level: usize,
    len: usize,
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
    rng: u64,
}

impl TyrusSkipList {
    fn compare_bytes(&self, a: &[u8], b: &[u8]) -> i32 {
        match self.compare {
            Some(f) => unsafe { f(a.as_ptr(), b.as_ptr()) },
            None => 0,
        }
    }

    fn node_less(&self, idx: usize, key: &[u8]) -> bool {
        self.compare_bytes(&self.nodes[idx].key, key) < 0
    }

    fn random_level(&mut self) -> usize {
        let mut lvl = 0;
        while lvl < MAX_LEVEL {
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 7;
            self.rng ^= self.rng << 17;
            if self.rng & 1 == 0 {
                break;
            }
            lvl += 1;
        }
        lvl
    }

    fn allocate_node(&mut self, key: &[u8], new_level: usize) -> usize {
        match self.free.pop() {
            Some(recycled) => {
                self.nodes[recycled] = SkipNode {
                    key: key.to_vec(),
                    next: vec![NONE; new_level + 1],
                };
                recycled
            }
            None => {
                self.nodes.push(SkipNode {
                    key: key.to_vec(),
                    next: vec![NONE; new_level + 1],
                });
                self.nodes.len() - 1
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_skip_list_new(
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
) -> *mut TyrusSkipList {
    let sentinel = SkipNode {
        key: Vec::new(),
        next: vec![NONE; MAX_LEVEL + 1],
    };
    let seed = SEED.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
    let list = Box::new(TyrusSkipList {
        nodes: vec![sentinel],
        free: Vec::new(),
        head: 0,
        level: 0,
        len: 0,
        element_size,
        compare,
        rng: if seed == 0 {
            0x853C_49E6_748F_EA9B
        } else {
            seed
        },
    });
    Box::into_raw(list)
}

#[unsafe(no_mangle)]
#[allow(clippy::needless_range_loop)]
pub unsafe extern "C" fn tyrus_skip_list_insert(
    list: *mut TyrusSkipList,
    element: *const u8,
) -> bool {
    unsafe {
        let list = &mut *list;
        let key = std::slice::from_raw_parts(element, list.element_size);
        let mut update = [list.head; MAX_LEVEL + 1];
        let mut current = list.head;
        for i in (0..=list.level).rev() {
            loop {
                let next = list.nodes[current].next[i];
                if next == NONE || !list.node_less(next, key) {
                    break;
                }
                current = next;
            }
            update[i] = current;
        }
        let first = list.nodes[current].next[0];
        if first != NONE && list.compare_bytes(&list.nodes[first].key, key) == 0 {
            return false;
        }
        let new_level = list.random_level();
        if new_level > list.level {
            for i in list.level + 1..=new_level {
                update[i] = list.head;
            }
            list.level = new_level;
        }
        let idx = list.allocate_node(key, new_level);
        for i in 0..=new_level {
            let upd = update[i];
            let link = list.nodes[upd].next[i];
            list.nodes[idx].next[i] = link;
            list.nodes[upd].next[i] = idx;
        }
        list.len += 1;
        true
    }
}

#[unsafe(no_mangle)]
#[allow(clippy::needless_range_loop)]
pub unsafe extern "C" fn tyrus_skip_list_remove(
    list: *mut TyrusSkipList,
    element: *const u8,
) -> bool {
    unsafe {
        let list = &mut *list;
        let key = std::slice::from_raw_parts(element, list.element_size);
        let mut update = [list.head; MAX_LEVEL + 1];
        let mut current = list.head;
        for i in (0..=list.level).rev() {
            loop {
                let next = list.nodes[current].next[i];
                if next == NONE || !list.node_less(next, key) {
                    break;
                }
                current = next;
            }
            update[i] = current;
        }
        let target = list.nodes[current].next[0];
        if target == NONE || list.compare_bytes(&list.nodes[target].key, key) != 0 {
            return false;
        }
        for i in 0..=list.level {
            if list.nodes[update[i]].next[i] == target {
                let link = list.nodes[target].next[i];
                list.nodes[update[i]].next[i] = link;
            }
        }
        while list.level > 0 && list.nodes[list.head].next[list.level] == NONE {
            list.level -= 1;
        }
        list.free.push(target);
        list.len -= 1;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_skip_list_contains(
    list: *const TyrusSkipList,
    element: *const u8,
) -> bool {
    unsafe {
        let list = &*list;
        let key = std::slice::from_raw_parts(element, list.element_size);
        let mut current = list.head;
        for i in (0..=list.level).rev() {
            loop {
                let next = list.nodes[current].next[i];
                if next == NONE || !list.node_less(next, key) {
                    break;
                }
                current = next;
            }
        }
        let first = list.nodes[current].next[0];
        first != NONE && list.compare_bytes(&list.nodes[first].key, key) == 0
    }
}

fn extreme(list: &TyrusSkipList, want_max: bool) -> *mut u8 {
    if list.len == 0 {
        return ptr::null_mut();
    }
    let mut current = list.nodes[list.head].next[0];
    if want_max {
        loop {
            let next = list.nodes[current].next[0];
            if next == NONE {
                break;
            }
            current = next;
        }
    }
    let storage = unsafe { malloc(list.element_size.max(1)).cast::<u8>() };
    if storage.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        std::ptr::copy_nonoverlapping(list.nodes[current].key.as_ptr(), storage, list.element_size);
    }
    storage
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_skip_list_min(list: *const TyrusSkipList) -> *mut u8 {
    unsafe { extreme(&*list, false) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_skip_list_max(list: *const TyrusSkipList) -> *mut u8 {
    unsafe { extreme(&*list, true) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_skip_list_len(list: *const TyrusSkipList) -> usize {
    unsafe { (*list).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_skip_list_is_empty(list: *const TyrusSkipList) -> bool {
    unsafe { (*list).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_skip_list_clear(list: *mut TyrusSkipList) {
    unsafe {
        let list = &mut *list;
        list.nodes.clear();
        list.free.clear();
        list.nodes.push(SkipNode {
            key: Vec::new(),
            next: vec![NONE; MAX_LEVEL + 1],
        });
        list.level = 0;
        list.len = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_skip_list_free(list: *mut TyrusSkipList) {
    unsafe {
        if !list.is_null() {
            drop(Box::from_raw(list));
        }
    }
}
