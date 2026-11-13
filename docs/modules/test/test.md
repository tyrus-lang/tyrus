---
title: test / it
description: Register a named test.
---

# test / it

<SignatureBlock label="Signature">test(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Signature">it(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Signature">test(name: string, callback: () =&gt; Promise&lt;void&gt;): void</SignatureBlock>

`test` and `it` are the same function under two names. The callback runs once. The test passes when the callback returns and fails when an assertion fails or the callback throws; the failure message is printed under the test name.

A callback cannot assign to a binding declared outside it, because the callback outlives the scope that declared the binding (`E0436`). Keep the setup a test needs inside that test, or in a `beforeEach` that prepares state the test reads through a function.

<!-- @include: ../../.fragments/modules/test/test.md -->

## Async Tests

An `async` callback is awaited before the next hook or test runs. An assertion that fails after an `await` fails the test like any other, and so does a rejected promise.

<!-- @include: ../../.fragments/modules/test/test-async.md -->
