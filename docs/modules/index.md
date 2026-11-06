---
title: Native Modules
description: Runtime APIs exposed through stable tyrus:* import paths.
---

# Native Modules

Native modules expose runtime functionality through stable `tyrus:*` import paths. Their APIs are concise, and their implementations are linked into native binaries. Every module is part of the standard distribution, no package manager, no npm install, no external runtime.

## Available Modules

| Module | Purpose |
|--------|---------|
| [`tyrus:algorithm`](./algorithm/) | Native data structures and algorithms for sorting, searching, graphs, trees, and collections. |
| [`tyrus:archive`](./archive/) | Create, inspect, and safely extract ZIP, TAR, and TAR.GZ archives. |
| [`tyrus:cli`](./cli/) | Interactive terminal prompt helpers. |
| [`tyrus:compress`](./compress/) | Compress and decompress bytes with Gzip, Deflate, raw Deflate, and Brotli. |
| [`tyrus:crypto`](./crypto/) | Cryptographic primitives: digest, HMAC, AES-256-GCM, Argon2, JWT, and secure random. |
| [`tyrus:db`](./db/) | Versioned SQLite schema snapshots, read-only schema checks, and the database error contract. |
| [`tyrus:encoding`](./encoding/) | Explicit UTF-8, UTF-16, hexadecimal, Base64, Base32, and radix conversion. |
| [`tyrus:fs`](./fs/) | Synchronous file reads and writes for native programs. |
| [`tyrus:http`](./http/) | HTTP server, route, response, status, and API-oriented helpers. |
| [`tyrus:math`](./math/) | Mathematical constants and functions. |
| [`tyrus:memory`](./memory/) | Explicit ownership, secure erasure, and compile-time layout queries. |
| [`tyrus:net`](./net/) | Address parsing, `Result`-based name resolution, asynchronous TCP and UDP networking, and capability-gated probes. |
| [`tyrus:os`](./os/) | Cross-platform operating system identification: platform, architecture, hostname, locale. |
| [`tyrus:process`](./process/) | Shell-free subprocesses with bounded standard byte streams. |
| [`tyrus:simd`](./simd/) | Vectorized addition of equal-length `f32[]` and `f64[]` arrays with recoverable `Result` errors. |
| [`tyrus:stream`](./stream/) | Bounded progressive transport with capability-separated reading and writing. |
| [`tyrus:task`](./task/) | Cooperative async task management: cancellation and timed suspension. |
| [`tyrus:temporal`](./temporal/) | Native instants, calendar values, durations, and IANA time zones. |
| [`tyrus:test`](./test/) | Test suites, assertions, hooks, and modifiers run by `tyrus test`. |
| [`tyrus:utils`](./utils/) | Native text helpers such as ANSI styling and FIGlet banners. |

## Import Style

<!-- @include: ../.fragments/modules/import-style.md -->

`tyrus:*` imports are resolved by the compiler as built-in modules.

## Using Modules Together

Native modules compose naturally. A typical CLI project might read config from the filesystem, prompt for input, and apply styles:

<!-- @include: ../.fragments/modules/using-modules-together.md -->

## Runtime Boundary

Native modules should document their public imports, return types, error behavior, and diagnostics before being considered stable. APIs that print terminal UI chrome use stderr so stdout remains reserved for normal program output.
