# Closed: ASan stack overflow while generating an HTTP reply handler

## Resolution

Closed on 2026-08-30. Fluent HTTP reply receivers are now lowered iteratively: codegen collects `setCookie`, `clearCookie`, and `status` calls, emits the base reply once, and replays the operations in source order. This removes one nested `emit_expr` frame per method without adding runtime state or changing evaluation order.

## Reproducer

```console
ASAN_OPTIONS=detect_leaks=0 ./scripts/sanitizers --workspace --package tyrus-codegen --lib tests::emits_http_reply_status_send_handlers -- --exact --nocapture
```

The instrumented test passes with the normal test-thread stack and without `RUST_MIN_STACK`. Leak detection is disabled only because LeakSanitizer cannot run under the development sandbox's `ptrace`; AddressSanitizer remains enabled and reports no memory error.

## Regression coverage

`tests::emits_http_reply_status_send_handlers` compiles `reply.setCookie(...).clearCookie(...).status(...).send(...)` and verifies the emitted cookie, status, response validation, and body paths. The isolated ASan run passes with 390 codegen tests filtered out and the target test executed exactly once.
