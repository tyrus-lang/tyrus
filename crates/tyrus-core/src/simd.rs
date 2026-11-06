#[path = "simd/simd.rs"]
mod builtins;

pub(crate) use builtins::simd_builtins;
pub use builtins::{ADD_TARGET, DIV_TARGET, MUL_TARGET, SUB_TARGET};
