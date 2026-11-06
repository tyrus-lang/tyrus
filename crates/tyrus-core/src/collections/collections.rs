//! The hash maps the compiler runs on.
//!
//! The standard library hashes with SipHash and a per-process random seed. For
//! a compiler that is the wrong trade: the keys are short identifiers, the maps
//! are never exposed to untrusted input, and hashing them showed up as 8,1% of
//! a `tyrus check` and 7,7% of a `tyrus build` in the callgrind profile. These
//! aliases swap in the hasher `rustc` itself uses, which also makes iteration
//! order stable across runs instead of reshuffling on every process start.

pub type HashMap<K, V> = std::collections::HashMap<K, V, rustc_hash::FxBuildHasher>;
pub type HashSet<T> = std::collections::HashSet<T, rustc_hash::FxBuildHasher>;
