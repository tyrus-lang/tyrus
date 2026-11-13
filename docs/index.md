---
title: English Documentation
description: Tyrus language documentation in English.
layout: home
hero:
  text: '<span class="hero-line">TypeScript syntax</span><span class="hero-line">Rust semantics</span><span class="hero-subtitle">A programming language made for devs</span>'
  image:
    src: /tyrus-logo.png
    alt: Tyrus logo
  actions:
    - theme: brand
      text: Getting Started
      link: /docs/getting-started
    - theme: alt
      text: Playground
      link: https://playground.tyruslang.org
    - theme: alt
      text: Explore Modules
      link: /modules/
features:
  - title: Native binaries
    details: Native compilation of .tyr files with LLVM and a runtime included in the executable.
  - title: Safety by default
    details: Ownership, borrowing, Non-Lexical Lifetimes, numeric bounds, validators, and exhaustiveness enforced before production.
  - title: Enterprise API tooling
    details: Scaffolding, formatter, linter, LSP, REPL, web playground, sealed environments, and 20 native modules ship together.
---

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Compiled backend language</p>
<h2>Write familiar .tyr code</h2>
<p>Tyrus uses concise syntax in <code>.tyr</code> files, then gives it native execution, deterministic memory semantics with NLL, precise diagnostics, and backend-focused tooling.</p>

</div>
<div>

<!-- @include: ./.fragments/home/http-controller.md -->

</div>
</section>

<section class="tyrus-home-section">
<p class="tyrus-home-eyebrow">Language superpowers</p>
<div class="tyrus-home-glow-grid">
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12l2 2l4-4"/></g></svg></template>
<template #title>Validator decorations</template>
<template #description>Built-in rules like @email, @minLength, and @min, plus custom rules declared with validator function. Declarative validation with zero boilerplate.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 2L3 14h9l-1 8l10-12h-9l1-8z"/></svg></template>
<template #title>Compile-time comptime</template>
<template #description>Execute pure logic at compilation time with comptime(...). Precompute lookup tables, math constants, and configurations with zero runtime overhead.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></g></svg></template>
<template #title>Sealed env and mirror</template>
<template #description>Encrypt .env into target/&lt;profile&gt;/.env.sealed with binary fingerprints. The .env.example mirror automatically stays in sync on every change.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0a2.34 2.34 0 0 0 3.319 1.915a2.34 2.34 0 0 1 2.33 4.033a2.34 2.34 0 0 0 0 3.831a2.34 2.34 0 0 1-2.33 4.033a2.34 2.34 0 0 0-3.319 1.915a2.34 2.34 0 0 1-4.659 0a2.34 2.34 0 0 0-3.32-1.915a2.34 2.34 0 0 1-2.33-4.033a2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915"/><circle cx="12" cy="12" r="3"/></g></svg></template>
<template #title>Types</template>
<template #description>Integers and floats u8..u128, i8..i128, f32..f128, decimal, boolean, string, char, and first-class Result&lt;T, E&gt; types.</template>
</GlowCard>
</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Deterministic memory safety</p>
<h2>Ownership, borrowing, and NLL.</h2>
<p>Memory safety without a garbage collector. Non-Copy types move on assignment, while borrows are verified at compile time. With Non-Lexical Lifetimes (NLL), borrows expire after their last actual use, freeing variables for immediate mutation without waiting for scope exit.</p>
</div>
<div>

<!-- @include: ./.fragments/home/ownership-nll.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Scope-based cleanup</p>
<h2>Run cleanup always, or only on errors.</h2>
<p><code>defer</code> runs when the scope exits. <code>errdefer</code> runs only when a <code>Result</code> or optional return exits with an error, making rollback paths explicit.</p>
<p><a href="/language/control-flow#defer-and-errdefer">Learn about deferred cleanup</a></p>
</div>
<div>

<!-- @include: ./.fragments/home/defer-errdefer.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Exhaustive pattern matching</p>
<h2>Match any shape. Narrow every branch.</h2>
<p>Tagged unions, algebraic enum variants, and <code>match</code> with guard clauses give you exhaustive control flow. The compiler ensures every variant is handled and narrows types inside each arm.</p>
</div>
<div>

<!-- @include: ./.fragments/home/pattern-matching.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">No null. No undefined. Just <code>T?</code></p>
<h2>Absence is a state, not a value.</h2>
<p>A bare <code>return;</code> produces absence. <code>?.</code> and <code>??</code> propagate it safely. The compiler forces you to handle the absent case before the value reaches production, no runtime surprises.</p>
</div>
<div>

<!-- @include: ./.fragments/home/optionals.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Explicit success and failure</p>
<h2>Represent recoverable errors with <code>Result</code>.</h2>
<p><code>Result&lt;T, E&gt;</code> carries either an <code>Ok</code> value or an <code>Err</code> value. Match both variants or propagate an error with <code>?</code>.</p>
<p><a href="/language/types#result-values">Learn about Result</a></p>
</div>
<div>

<!-- @include: ./.fragments/home/result.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Handle errors at the call site</p>
<h2>Keep the success path clear with <code>onfail</code>.</h2>
<p>Attach an error handler to a <code>Result</code> initializer. The handler sees the error, while the binding holds the success value when execution continues.</p>
<p><a href="/language/types#inline-failure-handling">Learn about onfail</a></p>
</div>
<div>

<!-- @include: ./.fragments/home/onfail.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Pipe operator</p>
<h2>Data flows top to bottom.</h2>
<p>The <code>|&gt;</code> operator threads values through function chains. The <code>%</code> topic token lets you place the piped value exactly where it belongs, no intermediate variables, no nesting.</p>
</div>
<div>

<!-- @include: ./.fragments/home/pipe-operator.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Native asynchronous code</p>
<h2>Write async functions and await their results.</h2>
<p><code>async</code> functions return <code>Promise&lt;T&gt;</code>. Use <code>await</code> to read the value when the asynchronous operation completes.</p>
<p><a href="/language/async-await">Learn about async and await</a></p>
</div>
<div>

<!-- @include: ./.fragments/home/async-await.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">High-performance numeric kernels</p>
<h2>Algebraic floating-point methods.</h2>
<p>Primitives <code>f32</code> and <code>f64</code> provide <code>algebraicAdd</code>, <code>algebraicSub</code>, <code>algebraicMul</code>, <code>algebraicDiv</code>, and <code>algebraicRem</code>. They explicitly grant the compiler permission to reassociate floating-point calculations for SIMD vectorization and hardware-accelerated math kernels.</p>
</div>
<div>

<!-- @include: ./.fragments/home/algebraic-float.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Production secret protection</p>
<h2>Sealed env and auto-synced .env.example.</h2>
<p>Setting <code>security.envSealed</code> encrypts <code>.env</code> into <code>target/&lt;profile&gt;/.env.sealed</code> with asymmetric keys and embeds a binary fingerprint so containers never deploy plaintext secrets. In development, the compiler automatically mirrors keys and comments into <code>.env.example</code> with values cleared.</p>
</div>
<div>

<!-- @include: ./.fragments/home/sealed-env.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Typed ORM and ODM</p>
<h2>Use one Model API with SQL and MongoDB.</h2>
<p>Define models as typed interfaces and query them asynchronously. The same Model API supports relational databases and MongoDB, with recoverable failures returned as <code>Result</code>.</p>
<p><a href="/modules/db/">Explore the database module</a></p>
</div>
<div>

<!-- @include: ./.fragments/home/orm-odm.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Compile-time evaluation</p>
<h2>Run code during compilation. Zero runtime cost.</h2>
<p>Functions declared with <code>comptime</code> execute during compilation and embed their result as static scalars or literals directly into the binary. Calculate lookup tables, mathematical constants, and configurations with complete type safety.</p>
</div>
<div>

<!-- @include: ./.fragments/home/comptime.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Interactive exploration</p>
<h2>Try in the terminal. Run in the browser.</h2>
<p>Explore Tyrus instantly without project setup. <code>tyrus repl</code> provides an interactive console with in-process LLVM JIT evaluation, multi-line editing, session save/restore, and <code>:type</code> inspection. Or open the <strong>Web Playground</strong> at <a href="https://playground.tyruslang.org" target="_blank" rel="noreferrer">playground.tyruslang.org</a> to write, compile to WebAssembly, and run code directly in your browser with shareable links.</p>
</div>
<div>

<!-- @include: ./.fragments/home/repl-session.md -->

</div>
</section>

<section class="tyrus-home-section">
<p class="tyrus-home-eyebrow">A language toolchain, not just a parser</p>
<div class="tyrus-home-glow-grid">
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m4 7l8-4l8 4v10l-8 4l-8-4zm8 14V11m8-4l-8 4l-8-4"/></svg></template>
<template #title>Interactive REPL & CLI</template>
<template #description>tyrus repl for in-process LLVM JIT evaluation, tyrus dev for live reload, and tyrus fmt for zero-config formatting.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m21.73 18l-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3M12 9v4m0 4h.01"/></svg></template>
<template #title>Diagnostics & tyrus explain</template>
<template #description>Stable error codes with tyrus explain, accurate ranges, help messages, runtime failure examples, and editor Quick Fixes.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497zM15 5l4 4"/></svg></template>
<template #title>Editors powered by LSP</template>
<template #description>Completion, hover, signature help, inlay hints, semantic tokens, and code actions for VS Code, Neovim, Zed, IntelliJ, and Kate.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 2v6a2 2 0 0 0 .245.96l5.51 10.08A2 2 0 0 1 18 22H6a2 2 0 0 1-1.755-2.96l5.51-10.08A2 2 0 0 0 10 8V2M6.453 15h11.094M8.5 2h7"/></svg></template>
<template #title>Web Playground & tests</template>
<template #description>Run in the browser via WebAssembly at playground.tyruslang.org, and run local test suites with tyrus test for *.test.tyr files.</template>
</GlowCard>
</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Source formatting</p>
<h2>Tyrus Formatter</h2>
<p><code>tyrus fmt</code> formats `.tyr` files consistently and applies the options in the project's <code>tyrus-fmt.json</code>.</p>
<p><a href="/tooling/formatter">Read the Formatter guide</a></p>
</div>
<div>

<!-- @include: ./.fragments/home/formatter-config.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Code analysis</p>
<h2>Tyrus Lint</h2>
<p><code>tyrus lint</code> reports style, correctness, safety, performance, and architecture issues. Set each rule's severity in the project's <code>tyrus-lint.json</code>.</p>
<p><a href="/tooling/linter">Read the Lint guide</a></p>
</div>
<div>

<!-- @include: ./.fragments/home/lint-config.md -->

</div>
</section>

<section class="tyrus-home-section">
<p class="tyrus-home-eyebrow">Batteries included</p>
<h2>Built-in native modules.</h2>
<p>Everything required to build resilient backend services, CLI tools, and systems software is built into the Tyrus runtime.</p>
<div class="tyrus-home-grid">
<a class="tyrus-home-card" href="/modules/algorithm/">
<strong>tyrus:algorithm</strong>
<span>Over 50 built-in algorithms and data structures: Dijkstra, A*, Trie, and Red-Black Tree.</span>
</a>
<a class="tyrus-home-card" href="/modules/archive/">
<strong>tyrus:archive</strong>
<span>Tar and Zip archive creation, inspection, and extraction.</span>
</a>
<a class="tyrus-home-card" href="/modules/cli/">
<strong>tyrus:cli</strong>
<span>Interactive terminal prompts: select, multiselect, password, confirm, and input.</span>
</a>
<a class="tyrus-home-card" href="/modules/compress/">
<strong>tyrus:compress</strong>
<span>Zstandard, Brotli, Gzip, and Deflate compression and decompression streams.</span>
</a>
<a class="tyrus-home-card" href="/modules/crypto/">
<strong>tyrus:crypto</strong>
<span>Argon2, SHA-256, SHA-512, AES-GCM, HMAC, JWT, UUID v7, and CSPRNG.</span>
</a>
<a class="tyrus-home-card" href="/modules/db/">
<strong>tyrus:db</strong>
<span>Integrated database client and ORM for PostgreSQL, MySQL, SQLite, and MongoDB.</span>
</a>
<a class="tyrus-home-card" href="/modules/encoding/">
<strong>tyrus:encoding</strong>
<span>Hex, Base64, URL encoding, Varint, CSV, and JSON native codecs.</span>
</a>
<a class="tyrus-home-card" href="/modules/fs/">
<strong>tyrus:fs</strong>
<span>Async and sync filesystem, atomic writes, streaming, glob, and file watching.</span>
</a>
<a class="tyrus-home-card" href="/modules/http/">
<strong>tyrus:http</strong>
<span>HTTP/1.1 and HTTP/2 server, automatic routes, Swagger, and WebSocket.</span>
</a>
<a class="tyrus-home-card" href="/modules/math/">
<strong>tyrus:math</strong>
<span>Constants, trigonometry, number theory, geometry, and combinatorics.</span>
</a>
<a class="tyrus-home-card" href="/modules/memory/">
<strong>tyrus:memory</strong>
<span>Deterministic memory operations: sizeOf, alignOf, zeroize, swap, and replace.</span>
</a>
<a class="tyrus-home-card" href="/modules/net/">
<strong>tyrus:net</strong>
<span>TCP and UDP sockets, DNS resolution, reverse lookup, ping, and traceroute.</span>
</a>
<a class="tyrus-home-card" href="/modules/os/">
<strong>tyrus:os</strong>
<span>System metadata, CPU cores, memory stats, temperatures, and native notifications.</span>
</a>
<a class="tyrus-home-card" href="/modules/process/">
<strong>tyrus:process</strong>
<span>Child processes, command execution, signals, and resource monitoring.</span>
</a>
<a class="tyrus-home-card" href="/modules/simd/">
<strong>tyrus:simd</strong>
<span>Portable vector math intrinsics for high-throughput numeric operations.</span>
</a>
<a class="tyrus-home-card" href="/modules/stream/">
<strong>tyrus:stream</strong>
<span>Composable binary and text streaming primitives for high-performance I/O.</span>
</a>
<a class="tyrus-home-card" href="/modules/task/">
<strong>tyrus:task</strong>
<span>Async runtime, multi-thread spawn, channels, mutex, rwlock, and cancellation.</span>
</a>
<a class="tyrus-home-card" href="/modules/temporal/">
<strong>tyrus:temporal</strong>
<span>Modern date, time, duration, and timezone handling with ergonomic syntax.</span>
</a>
<a class="tyrus-home-card" href="/modules/test/">
<strong>tyrus:test</strong>
<span>Built-in test runner with describe, it, and expect for *.test.tyr files.</span>
</a>
<a class="tyrus-home-card" href="/modules/utils/">
<strong>tyrus:utils</strong>
<span>Terminal text styling with styleText, ANSI colors, and ASCII FIGlet banners.</span>
</a>
</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Safety feedback while you type</p>
<h2>Errors are part of the product experience.</h2>
<p>Compiler and linter diagnostics flow through the same analysis pipeline used by the CLI and the Language Server, so the terminal and editor agree on what is wrong and how to fix it.</p>
</div>
<div>

::: danger error
<!-- @include: ./.fragments/home/type-error.md -->
:::

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Start with one command</p>

<!-- @include: ./.fragments/home/new-command.md -->

</div>
<div>
<p class="tyrus-home-eyebrow">Run with one command</p>

<!-- @include: ./.fragments/home/run-command.md -->

</div>
</section>

<section class="tyrus-home-section">
<p class="tyrus-home-eyebrow">Why the name</p>
<h2>A rock and a god.</h2>
<p>Tyrus is the Latin spelling of Tyre, the Phoenician city whose name means "rock". Tyr is the Norse god of war, justice, and courage.</p>
<p>But the name is a combination of the initials of <span class="tyrus-home-strong">Ty</span>peScript and <span class="tyrus-home-strong">Rus</span>t.</p>
</section>
