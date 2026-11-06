# Resolved: HTTP struct layout server does not answer

## Status

Opened 2026-08-25, resolved 2026-09-04. The reproducer passes, and passed five consecutive isolated runs at ~3.3s each, well inside the test's own 20-second deadline.

## Reproducer

```console
cargo test -p tyrus --test struct_layout_runtime every_field_survives_an_http_round_trip -- --exact
```

## What it looked like

The test built and started the generated `struct-layout-http` binary, but `request_body` never obtained a response from `127.0.0.1` within its deadline and returned `None`. The record noted that the test discarded the child's stdout and stderr, so the failure never revealed whether the process exited early, panicked, or stayed alive without accepting connections.

## Why it passes now

The record is closed on evidence that the test passes reliably, not on a demonstrated single cause. The `tyrus-http` branch changed several things this reproducer depends on, and any of them could account for it:

- The response slot the route hands to generated code became independently owned (`ResponseSlot` in `crates/tyrus-runtime/src/http.rs`). Before that, a request whose future was dropped freed the slot while an async handler was still writing to it, which is a use-after-free that can present as a hang.
- Registering an inline `async` handler from inside a function emitted invalid IR, because the async path of `emit_http_inline_handler` did not restore the builder's block. Everything after the route registration was appended to the handler.
- Text channels (`params`, `query`, `headers`, `cookies`) are read according to the declared schema rather than by guessing a JSON type from the characters.

## If it returns

Capture the child's exit status and stderr while polling for the response, and distinguish an early exit from a live server that never binds. Then move this record back to `tests/leaks/known/`.
