---
title: Modifiers
description: Skip, focus, and plan tests.
---

# Modifiers

<SignatureBlock label="Signature">test.skip(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Signature">test.only(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Signature">test.todo(name: string): void</SignatureBlock>

<SignatureBlock label="Signature">describe.skip(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Signature">describe.only(name: string, callback: () =&gt; void): void</SignatureBlock>

`test.skip` lists a test as skipped without running it; its callback still has to compile. `describe.skip` skips every test in the suite, and the suite hooks do not run.

`test.todo(name)` lists a test that has not been written yet. Skipped tests and todos are counted separately in the summary.

`it` accepts the same modifiers, and so does an aliased import such as `import { test as check } from "tyrus:test"`. `test.concurrent` and `test.fails` are not implemented yet and are reported as compile errors.

<!-- @include: ../../.fragments/modules/test/modifiers.md -->

## Focus with `.only`

When any test or suite in a program is focused with `.only`, only focused tests run and the others count as skipped. `.skip` wins over `.only`: focusing never runs a test that was skipped on purpose.

<!-- @include: ../../.fragments/modules/test/modifiers-only.md -->

## One Test per Case

`test.each(name, cases, callback)` registers one test per case, named `name [0]`, `name [1]`, and so on. The callback takes one case; written inline, its parameter type is inferred from the cases. The cases must be an array literal, because they are unrolled when the program is compiled.

<!-- @include: ../../.fragments/modules/test/each.md -->
