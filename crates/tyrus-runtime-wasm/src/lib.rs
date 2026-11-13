//! WASM-compatible runtime services for Tyrus programs.
//!
//! This crate provides the same C ABI as `tyrus-runtime` but compiled for
//! `wasm32-unknown-unknown`. Everything that is pure computation is the *same
//! source* as the native runtime, pulled in with `#[path]`; only what genuinely
//! cannot run in a browser sandbox, the OS, network, filesystem and async
//! subsystems, is stubbed in [`stubs`].
//!
//! The rule for adding to this crate: if a module would be a copy of a native
//! one, share it instead. A second copy is how the two drifted into 122
//! signature mismatches, each of which links a trapping stub instead of the
//! function the program called.

#![allow(
    clippy::missing_safety_doc,
    reason = "exported functions form an internal compiler/runtime C ABI"
)]

use std::ffi::{CString, c_void};
use std::ptr;

// wasm32 only: dlmalloc backs this allocator and is meant for the WASM/no-std
// target, so installing it as the global allocator on the host segfaults the
// process before `main`, which silently made every `cargo test` in this crate
// unrunnable (not even `--list` worked). On the host the system allocator is
// used instead, so the tests exercise the same code with a working heap.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: alloc::OomReporting = alloc::OomReporting;

#[path = "alloc/alloc.rs"]
pub mod alloc;
#[path = "handles/handles.rs"]
pub mod handles;
#[path = "output/output.rs"]
pub mod output;
#[path = "sandbox/sandbox.rs"]
pub mod sandbox;
#[path = "stubs/stubs.rs"]
pub mod stubs;
#[path = "types/types.rs"]
pub mod types;
#[path = "unavailable/unavailable.rs"]
pub mod unavailable;

#[path = "../../tyrus-runtime/src/abi/abi.rs"]
pub mod abi;
#[path = "../../tyrus-runtime/src/arrays/arrays.rs"]
pub mod arrays;
#[path = "../../tyrus-runtime/src/binary128/binary128.rs"]
pub mod binary128;
#[path = "../../tyrus-runtime/src/closure/closure.rs"]
pub mod closure;
#[path = "../../tyrus-runtime/src/decimal/decimal.rs"]
pub mod decimal;
#[path = "../../tyrus-runtime/src/encoding/encoding.rs"]
pub mod encoding;
#[path = "../../tyrus-runtime/src/json/json.rs"]
pub mod json;
#[path = "../../tyrus-runtime/src/map/map.rs"]
pub mod map;
#[path = "../../tyrus-runtime/src/math/math.rs"]
pub mod math;
#[path = "../../tyrus-runtime/src/memory/memory.rs"]
pub mod memory;
#[path = "../../tyrus-runtime/src/panic/panic.rs"]
pub mod panic;
#[path = "../../tyrus-runtime/src/set/set.rs"]
pub mod set;
#[path = "../../tyrus-runtime/src/strings/strings.rs"]
pub mod strings;
#[path = "../../tyrus-runtime/src/strings_core/strings_core.rs"]
mod strings_core;
#[path = "../../tyrus-runtime/src/temporal/temporal.rs"]
pub mod temporal;
#[path = "../../tyrus-runtime/src/validation/validation.rs"]
pub mod validation;

pub use abi::*;
pub use closure::*;
pub use decimal::*;
pub use encoding::*;
pub use handles::*;
pub use json::*;
pub use map::*;
pub use memory::*;
pub use panic::*;
pub use set::*;
pub use strings::*;
pub use temporal::*;
pub use unavailable::*;
pub use validation::*;

use types::text;
