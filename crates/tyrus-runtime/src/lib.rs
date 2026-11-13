//! Native runtime services exposed to generated tyrus programs through a C ABI.
//!
//! # Safety
//!
//! This crate is an internal ABI boundary, not a general-purpose Rust API. Generated code must pass non-null, correctly aligned pointers to live values of the declared ABI type; buffers must remain valid for the complete call, their lengths and capacities must describe the same allocation, and ownership may be transferred only by functions whose contract explicitly consumes or returns it. Calling these exports manually without preserving those invariants can cause undefined behavior.

#![allow(
    clippy::missing_safety_doc,
    reason = "the exported functions form an internal compiler/runtime C ABI whose pointer invariants are enforced by generated code"
)]

use axum::Router;
use axum::body::{Body, Bytes, to_bytes};
use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue, Method, Response, StatusCode};
use axum::routing::{MethodRouter, connect, delete, get, head, options, patch, post, put, trace};
use std::ffi::CString;
use std::ffi::c_void;
use std::net::SocketAddr;
use std::ptr;
use std::time::Duration;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, Any, CorsLayer, ExposeHeaders};

// This crate is linked as a staticlib into both the `tyrus` compiler binary and every binary
// generated for a user program, so declaring the global allocator once here covers both.
// On Unix, `override` also replaces raw libc allocation symbols used by generated LLVM IR.
// A static mimalloc build cannot override those symbols on Windows. Use the CRT for Rust and
// generated allocations there, so an FFI payload allocated by `Box` can be released by `free`
// and a generated allocation can be released by Rust without crossing allocator heaps.
//
// The `system-allocator` feature opts back out, and `scripts/leak-check*` build with it for a
// reason: valgrind's memcheck cannot see through mimalloc. mimalloc serves allocations from its own
// mmap'd arena, which memcheck never records, while the `free` calls still land on memcheck's
// interposed symbol, every release then reports `Invalid free()` against an address memcheck
// believes was never allocated. A leak corpus run that way is not merely noisy, it is blind: real
// leaks cannot be distinguished from that noise. Linking the crate without registering it here is
// enough to restore a clean memcheck run (verified: 0 errors), so the feature only gates the
// global allocator declaration.
#[cfg(all(not(feature = "system-allocator"), not(windows)))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(all(not(feature = "system-allocator"), windows))]
mod windows_crt_allocator {
    use std::alloc::{GlobalAlloc, Layout};
    use std::ffi::c_void;

    unsafe extern "C" {
        fn _aligned_malloc(size: usize, alignment: usize) -> *mut c_void;
        fn _aligned_realloc(pointer: *mut c_void, size: usize, alignment: usize) -> *mut c_void;
        fn _aligned_free(pointer: *mut c_void);
    }

    pub(super) struct WindowsCrtAllocator;

    unsafe impl GlobalAlloc for WindowsCrtAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            if layout.align() > 16 {
                unsafe { _aligned_malloc(layout.size().max(1), layout.align()).cast() }
            } else {
                unsafe { libc::malloc(layout.size().max(1)).cast() }
            }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            if layout.align() > 16 {
                let pointer = unsafe { self.alloc(layout) };
                if !pointer.is_null() {
                    unsafe { pointer.write_bytes(0, layout.size()) };
                }
                pointer
            } else {
                unsafe { libc::calloc(1, layout.size().max(1)).cast() }
            }
        }

        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            if layout.align() > 16 {
                unsafe { _aligned_free(pointer.cast()) };
            } else {
                unsafe { libc::free(pointer.cast()) };
            }
        }

        unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            if layout.align() > 16 {
                unsafe { _aligned_realloc(pointer.cast(), new_size.max(1), layout.align()).cast() }
            } else {
                unsafe { libc::realloc(pointer.cast(), new_size.max(1)).cast() }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        #[repr(align(64))]
        struct Aligned([u8; 64]);

        #[test]
        fn rust_and_generated_code_can_release_each_others_crt_allocations() {
            unsafe {
                let rust_value = Box::into_raw(Box::new(42_u64));
                libc::free(rust_value.cast());

                let generated_value = libc::malloc(std::mem::size_of::<u64>()).cast::<u64>();
                assert!(!generated_value.is_null());
                generated_value.write(42);
                assert_eq!(*Box::from_raw(generated_value), 42);

                let aligned = Box::new(Aligned([7; 64]));
                assert_eq!(((&*aligned) as *const Aligned as usize) % 64, 0);
                assert_eq!(aligned.0[0], 7);
            }
        }
    }
}

#[cfg(all(not(feature = "system-allocator"), windows))]
#[global_allocator]
static GLOBAL: windows_crt_allocator::WindowsCrtAllocator =
    windows_crt_allocator::WindowsCrtAllocator;

#[path = "abi/abi.rs"]
mod abi;
#[path = "archive/archive.rs"]
mod archive;
#[cfg(feature = "fuzzing")]
#[path = "archive_fuzz/archive_fuzz.rs"]
pub mod archive_fuzz;
#[path = "arrays/arrays.rs"]
mod arrays;
#[path = "async_payload/async_payload.rs"]
mod async_payload;
#[path = "avl_tree/avl_tree.rs"]
mod avl_tree;
#[path = "binary128/binary128.rs"]
mod binary128;
#[path = "binary_heap/binary_heap.rs"]
mod binary_heap;
#[path = "binomial_heap/binomial_heap.rs"]
mod binomial_heap;
#[path = "bitset/bitset.rs"]
mod bitset;
#[path = "bloom_filter/bloom_filter.rs"]
mod bloom_filter;
#[path = "centroid/centroid.rs"]
mod centroid;
#[path = "channel/channel.rs"]
mod channel;
#[path = "cli/cli.rs"]
mod cli;
#[path = "closure/closure.rs"]
mod closure;
#[path = "codec/codec.rs"]
mod codec;
#[cfg(feature = "fuzzing")]
#[path = "codec_fuzz/codec_fuzz.rs"]
pub mod codec_fuzz;
#[path = "compress/compress.rs"]
mod compress;
#[path = "cors/cors.rs"]
mod cors;
#[path = "counter/counter.rs"]
mod counter;
#[path = "crypto/crypto.rs"]
mod crypto;
#[cfg(feature = "benchmarking")]
#[path = "crypto_bench/crypto_bench.rs"]
pub mod crypto_bench;
#[cfg(feature = "fuzzing")]
#[path = "crypto_fuzz/crypto_fuzz.rs"]
pub mod crypto_fuzz;
#[path = "db/db.rs"]
mod db;
#[path = "decimal/decimal.rs"]
mod decimal;
#[path = "deque/deque.rs"]
mod deque;
#[path = "double_linked_list/double_linked_list.rs"]
mod double_linked_list;
#[path = "encoding/encoding.rs"]
mod encoding;
#[path = "env/env.rs"]
mod env;
#[path = "executor/executor.rs"]
mod executor;
#[path = "fenwick_tree/fenwick_tree.rs"]
mod fenwick_tree;
#[path = "fetch_policy/fetch_policy.rs"]
mod fetch_policy;
#[path = "figlet/figlet.rs"]
mod figlet;
#[path = "flow/flow.rs"]
mod flow;
#[path = "fs/fs.rs"]
mod fs;
#[path = "graph/graph.rs"]
mod graph;
#[path = "hld/hld.rs"]
mod hld;
#[path = "http/http.rs"]
mod http;
#[path = "http_client/http_client.rs"]
mod http_client;
#[path = "http_dev_tls/http_dev_tls.rs"]
mod http_dev_tls;
#[path = "http_docs/http_docs.rs"]
mod http_docs;
#[path = "http_multipart/http_multipart.rs"]
mod http_multipart;
#[path = "http_tls/http_tls.rs"]
mod http_tls;
#[path = "http_websocket/http_websocket.rs"]
mod http_websocket;
#[path = "json/json.rs"]
mod json;
#[path = "lca/lca.rs"]
mod lca;
#[path = "linked_list/linked_list.rs"]
mod linked_list;
#[path = "map/map.rs"]
mod map;
#[path = "math/math.rs"]
mod math;
#[path = "memory/memory.rs"]
mod memory;
#[path = "mst/mst.rs"]
mod mst;
#[path = "mutex/mutex.rs"]
mod mutex;
#[path = "net/net.rs"]
mod net;
#[path = "net_probe/net_probe.rs"]
mod net_probe;
#[path = "net_resolve/net_resolve.rs"]
mod net_resolve;
#[path = "net_tcp/net_tcp.rs"]
mod net_tcp;
#[path = "net_udp/net_udp.rs"]
mod net_udp;
#[path = "os/os.rs"]
mod os;
#[path = "panic/panic.rs"]
mod panic;
#[path = "path/path.rs"]
mod path;
#[path = "paths/paths.rs"]
mod paths;
#[path = "print_lock/print_lock.rs"]
mod print_lock;
#[path = "process/process.rs"]
mod process;
#[path = "promise/promise.rs"]
mod promise;
#[path = "promise_capability/promise_capability.rs"]
mod promise_capability;
#[path = "promise_chains/promise_chains.rs"]
mod promise_chains;
#[path = "promise_combinators/promise_combinators.rs"]
mod promise_combinators;
#[path = "red_black_tree/red_black_tree.rs"]
mod red_black_tree;
#[path = "rwlock/rwlock.rs"]
mod rwlock;
#[path = "search/search.rs"]
mod search;
#[path = "segment_tree/segment_tree.rs"]
mod segment_tree;
#[path = "segment_tree_lazy/segment_tree_lazy.rs"]
mod segment_tree_lazy;
#[path = "semaphore/semaphore.rs"]
mod semaphore;
#[path = "set/set.rs"]
mod set;
#[path = "skip_list/skip_list.rs"]
mod skip_list;
#[path = "sorted_avl/sorted_avl.rs"]
mod sorted_avl;
#[path = "sorted_map/sorted_map.rs"]
mod sorted_map;
#[path = "sorted_set/sorted_set.rs"]
mod sorted_set;
#[path = "sorting/sorting.rs"]
mod sorting;
#[path = "sparse_table/sparse_table.rs"]
mod sparse_table;
#[path = "splay_tree/splay_tree.rs"]
mod splay_tree;
#[path = "stream/stream.rs"]
mod stream;
#[path = "string_search/string_search.rs"]
mod string_search;
#[path = "strings/strings.rs"]
mod strings;
#[path = "strings_core/strings_core.rs"]
mod strings_core;
#[path = "structure/structure.rs"]
mod structure;
#[path = "style/style.rs"]
mod style;
#[path = "suffix_array/suffix_array.rs"]
mod suffix_array;
#[path = "suffix_tree/suffix_tree.rs"]
mod suffix_tree;
#[path = "task_combinators/task_combinators.rs"]
mod task_combinators;
#[path = "temporal/temporal.rs"]
mod temporal;
#[path = "test/test.rs"]
mod test;
#[path = "treap/treap.rs"]
mod treap;
#[path = "trie/trie.rs"]
mod trie;
#[path = "types/types.rs"]
mod types;
#[path = "union_find/union_find.rs"]
mod union_find;
#[path = "validation/validation.rs"]
mod validation;

pub use abi::*;
pub use archive::{
    tyrus_archive_list, tyrus_archive_pack, tyrus_archive_pack_stream, tyrus_archive_unpack,
    tyrus_archive_unpack_stream,
};
pub use arrays::*;
pub use avl_tree::*;
pub use binary_heap::*;
pub use binomial_heap::*;
pub use bitset::*;
pub use bloom_filter::*;
pub use centroid::*;
pub use channel::*;
pub use cli::*;
pub use compress::{
    tyrus_compress_compress, tyrus_compress_compression_stream, tyrus_compress_decompress,
    tyrus_compress_decompression_stream,
};
pub use cors::*;
pub use counter::*;
pub use crypto::*;
pub use decimal::*;
pub use deque::*;
pub use double_linked_list::*;
pub use encoding::*;
pub use env::*;
pub use executor::*;
pub use fenwick_tree::*;
pub use figlet::*;
pub use flow::*;
pub use fs::*;
pub use graph::*;
pub use hld::*;
pub use http::*;
pub use json::*;
pub use lca::*;
pub use linked_list::*;
pub use map::*;
pub use memory::*;
pub use mst::*;
pub use net::*;
pub use net_probe::*;
pub use net_resolve::*;
pub use net_tcp::*;
pub use net_udp::*;
pub use os::*;
pub use panic::*;
pub use path::*;
pub use promise::*;
pub use red_black_tree::*;
pub use search::*;
pub use segment_tree::*;
pub use segment_tree_lazy::*;
pub use set::*;
pub use skip_list::*;
pub use sorted_map::*;
pub use sorted_set::*;
pub use sorting::*;
pub use sparse_table::*;
pub use splay_tree::*;
pub use string_search::*;
pub use strings::*;
pub use structure::*;
pub use style::*;
pub use suffix_array::*;
pub use suffix_tree::*;
pub use test::*;
pub use treap::*;
pub use trie::*;
pub use types::*;
pub use union_find::*;
pub use validation::*;

use types::text;

const MAX_REQUEST_BODY: usize = 2 * 1024 * 1024;

#[cfg(test)]
#[path = "../tests/unit/http_runtime.rs"]
mod tests;
