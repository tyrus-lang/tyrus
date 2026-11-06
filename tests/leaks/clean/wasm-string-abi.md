# Closed: runtime-built `string` values now cross the wasm32 C ABI

Not a leak, and there was no `.tyr` leak case: the program did not link, so there was nothing to measure. It remains recorded here as regression context because it blocked the playground.

The regression gate is `scripts/wasm --check`. It builds the real WASM staticlib and compiler, then links the Playground Fibonacci example plus a runtime call that both accepts and returns `string`.

## Resolution

Codegen now keeps its convenient `{ptr, i64}` string value internally and generates a private ABI thunk for every WASM runtime declaration that directly accepts or returns `string`. Each thunk spills arguments into Rust's `{ptr, u32, u32}` memory layout and passes their addresses; string returns use an out-parameter and are packed back into the generated representation. The conversion lives in `declare_libc_fn`, so every existing and future direct string boundary receives the same handling without target conditionals at individual call sites.

The duplicated `#[no_mangle]` on the WASM `tyrus_print_str` stub was also removed, eliminating the accompanying `unused attribute` warning.

## Former symptom

```
wasm-ld-17: error: function signature mismatch: tyrus_print_str
>>> defined as (i32, i64, i32) -> void in main.o
>>> defined as (i32, i32) -> void in libtyrus_runtime_wasm.a
```

`wasm-ld` is only warning. `--fatal-warnings` (`src/main.rs`) promotes it on purpose: otherwise `wasm-ld` links a stub that traps when called, and the mismatch reaches a user as an unexplained `unreachable` instead of a build error naming the function.

## What failed and what did not

Measured, not assumed:

| program | WASM link before the fix |
|---|---|
| `console.log("hello")` | **links** |
| `console.log('hello')` | **links** |
| `console.log(a + "llo")` | **fails** |
| ``console.log(`n=${n.toString()}`)`` | **fails** |

A string *literal* never reaches `tyrus_print_str`, its length is known at compile time and it prints through a different path. Every string built at run time does reach it, which is why concatenation and interpolation failed together while quotes of either kind were irrelevant. The quoting style was a red herring; the dividing line was compile-time versus run-time text.

## Cause

`TyrusString` is `{ ptr, bytes: u32, chars: u32 }`.

| target | size | how the C ABI passes it |
|---|---|---|
| x86-64 | 8 + 4 + 4 = **16 bytes** | two registers, which is exactly why codegen models it as `{ptr, i64}`; see the module documentation in `crates/tyrus-codegen/src/string_repr.rs` |
| wasm32 | 4 + 4 + 4 = **12 bytes** | **indirectly**: the caller spills it and passes the address |

Rust followed the WASM ABI while codegen flattened the struct into `(i32, i64)` the way it does natively. The two sides disagreed about how the value traveled until the boundary thunks were added.

This is a consequence of the representation change. While a `string` was a bare `*const c_char`, one pointer crossed both ABIs identically and the question never came up. The playground worked on `main` for that reason.

## Scope

Counted from the source, not estimated:

- **134** `declare_libc_fn` sites take a `string` as an argument
- **28** more return one

A given program only trips over the entry points it actually calls, so fixing one surfaces the next. The set to fix is all of them.

## Approaches considered

**A, pass indirectly on wasm.** This was implemented through generated boundary thunks rather than by changing `string_abi_type()` and every call site. It matches Rust, changes nothing natively, and centralizes the target conditional.

**B, flatten the boundary explicitly, on both targets.** Change the runtime signatures from `fn f(value: TyrusString)` to `fn f(ptr: *const u8, lengths: u64)` and have codegen pass the two fields it already has (`string_data_ptr` and the packed word). This removes the struct from the calling convention entirely, so no ABI has an opinion about it. Works unchanged on x86-64 (two scalars still travel in two registers, exactly as today) and needs no per-target branching, which also preserves the property that `tyrus-runtime` and `tyrus-runtime-wasm` compile from one shared source.

**B originally looked better**, but the thunk form of A removed its main drawback: there is one target-conditional implementation rather than target conditionals spread across all call sites. It also avoided changing the public runtime ABI on native targets.

The runtime parity test remains meaningful because the native and WASM runtime sources still expose the same Rust signatures; the new link gate covers the separate codegen-to-runtime boundary that source-level parity cannot observe.

## Why the link gate is required

Three guards exist and **none of them catches this**, all for the same reason, they stop before the link:

- `shared_symbols_agree_on_their_signature` compares the two runtimes' *declared* signatures against each other. Both are `TyrusString`; they agree. It cannot see what codegen emits.
- `crates/tyrus-codegen/tests/wasm_target_ir.rs` emits and verifies IR for both targets. The IR is valid; the disagreement is with the staticlib, which the test never links.
- `cargo check --workspace` compiles the crate. Compiling is not linking.

The error only exists once `wasm-ld` joins the two objects. `scripts/wasm --check` does that against the real staticlib and now uses the actual Playground Fibonacci shape, including its runtime-built interpolated string. It also calls a method returning `string`, covering both ABI directions.

## Validation

1. `cargo test -p tyrus-codegen --test wasm_target_ir` verifies all maintained Playground examples for both native and WASM IR.
2. `scripts/wasm --check` compiles and links the real Playground Fibonacci program against the current WASM staticlib.
3. The same link case exercises a runtime function returning `string`, so both indirect arguments and out-parameter returns remain covered.

## Already true, so it does not need redoing

- `crates/tyrus-runtime-wasm` compiles for `wasm32-unknown-unknown` against the new representation. Getting there needed two fixes that are in place: `tyrus-string` declares `malloc`/`free` itself instead of reaching through the `libc` crate, which has no such functions for a target with no libc; and the sandbox gained its own `tyrus_print_str`, writing the recorded byte count into the playground's buffers rather than through a `FILE*` that does not exist there.
- The 51 stub signatures that had drifted are aligned, and the parity test passes.
- `scripts/wasm` builds the staticlib. Nothing built it before, which is why the one the playground linked was weeks old, the first symptom, and the one that hid the real problem underneath.
