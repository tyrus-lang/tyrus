# Crypto FFI safety audit

This document records the invariants used by the native `tyrus:crypto` ABI. It is an internal self-review artifact, not evidence of an independent security audit. An independent reviewer must still validate these assumptions before release.

## Universal caller invariants

- Every non-null pointer must be correctly aligned, point to a live allocation of the declared layout, remain valid for the full call, and obey Rust aliasing rules.
- A byte pointer may be null only when its paired length is zero; every non-empty byte range must be readable for exactly its declared length.
- A `string` argument arrives by value as `{ pointer, packed lengths }`. Its pointer must reference readable storage of exactly the recorded byte length, holding valid UTF-8, which may include U+0000, since the length delimits the text rather than a terminator. The caller retains ownership; the runtime copies anything it needs to keep.
- An output `TyrusArray` pointer must reference writable initialized storage. On success, the runtime replaces it with `{ data, length, capacity }`, where non-empty `data` comes from the C allocator and `capacity == length`; generated array cleanup owns and frees that allocation.
- Option structs and every nested pointer they expose are borrowed for the duration of the call. String-array data points to `length` readable `string` values, not byte elements.
- A `DigestStream` pointer must originate from `tyrus_crypto_digest_create`, remain exclusively owned, and be passed to `tyrus_crypto_digest_stream_free` at most once. Generated code enforces lexical cleanup and disarms it after explicit `dispose()`.
- Promise pointers returned by crypto entry points follow the runtime promise retain/release contract. Async workers retain before escape, settle at most once, release their retained reference after settlement, and transfer fulfilled payload ownership through the registered destructor.

## Boundary inventory

| Boundary | Borrowed input | Owned output | Validation before dereference or allocation |
| --- | --- | --- | --- |
| UTF-8, digest, HMAC, equality | `string` values or `(ptr, len)` byte ranges | `string` or `TyrusArray` | Null/length, 64 MiB input limit, UTF-8, writable output |
| Random values | Scalar bounds | `string`, scalar, or `TyrusArray` | Range order, 16 MiB random-byte limit, entropy failure |
| AES-256-GCM | Data, 32-byte key, AAD, envelope | Versioned envelope, `string`, or `TyrusArray` | Null/length, exact key size, minimum envelope size, authenticated header/version/algorithm, authentication before plaintext exposure |
| Argon2id | Password bytes, PHC `string`, option structs, pepper bytes | Promise-owned string or boolean | 1 KiB password/pepper limits, 4 KiB PHC limit, bounded work factors, non-empty pepper, four-slot concurrency limit |
| JWT/JWK/JWKS | Claims/token/key/JWKS `string` values and option structs | `string` or promise-owned string | 1 MiB token/payload limit, 64 KiB key limit, algorithm allowlist, asymmetric public JWK requirement, metadata/key-family agreement, bounded option strings and arrays |
| Digest stream | Exclusive native handle and byte ranges | `TyrusArray` | Non-null handle, null/length input validation, writable output |

## Unsafe operation review

- Reading a `string`'s bytes is reached only after a null check and relies on the recorded byte length being the true extent of the allocation; native code cannot discover the allocation boundary on its own. This replaced `CStr::from_ptr`, which had to scan for a terminator and so could not distinguish a short string from an unterminated one.
- `slice::from_raw_parts` is reached only after rejecting a null non-empty pointer and lengths above the applicable limit; liveness, alignment, and readable allocation size remain caller obligations.
- `ptr::copy_nonoverlapping` copies from Rust-owned immutable slices into fresh non-overlapping C allocations whose exact size was checked before use.
- `malloc` results are checked for null before writes. Empty arrays use a null pointer without calling `malloc(0)`.
- `free` is used only for allocations returned by the same C allocator or payloads whose ABI contract transfers that ownership. A `string`'s buffer is released only when its ownership bit is set, which is what keeps a value over static storage (an error message, a literal) from ever reaching `free`.
- `Box::from_raw` is used only for live `DigestStream` handles created by `Box::into_raw`; generated ownership tracking prevents a second automatic drop after `dispose()`.
- Raw promise payload slots are allocated to their exact pointee size, initialized before fulfillment, and paired with the destructor matching the stored payload type.
- Thread-local synchronous errors contain only process-lifetime static text, are consumed exactly once, and never retain borrowed input or secret material. Their `string` values carry a clear ownership bit, so a caller that releases every string it holds still cannot free them.

## Secret lifetime

- Runtime-controlled copies of HMAC keys, AES keys, decrypted plaintext, Argon2 passwords, peppers, salts, and private/HMAC JWT keys use `Zeroizing` or explicit zeroization on error paths.
- The runtime cannot guarantee erasure of caller-owned values, compiler-created temporaries, allocator metadata, swapped pages, kernel buffers, third-party internal copies, or previously copied values.
- Public JWK and fetched JWKS paths reject symmetric `oct` keys. HMAC secrets must use the explicit symmetric-key argument and are never accepted as remotely fetched public-key material.

## Residual risks and required independent review

- No FFI callee can prove that a non-null raw pointer is live, sufficiently large, correctly aligned, or free from concurrent mutation. These properties depend on generated caller correctness and require end-to-end review of compiler codegen.
- Allocation-failure branches, a host clock earlier than the Unix epoch, poisoned global mutexes, and selected worker-failure branches need fault injection to exercise deterministically.
- HTTPS JWKS transport requires integration validation against controlled public endpoints and platform TLS/DNS implementations; unit tests cover parsing, policy, cache, single-flight, address classification, and pre-network rejection.
- Before release, an independent reviewer must inspect this inventory, the matching codegen declarations and cleanup paths, every `unsafe` block in `crypto.rs` and `crypto_bench.rs`, sanitizer/Valgrind results, fuzz corpora, and cryptographic protocol choices.

## Reproducible local checks

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo llvm-cov -p tyrus-runtime --lib --summary-only
cargo bench -p tyrus-runtime --bench crypto_allocations --features benchmarking
cargo bench -p tyrus-runtime --bench crypto_scalability --features benchmarking
cargo bench -p tyrus-runtime --bench crypto_timing --features benchmarking
```
