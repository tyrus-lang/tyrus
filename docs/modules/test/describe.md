---
title: describe
description: Group tests into a named suite.
---

# describe

<SignatureBlock label="Signature">describe(name: string, callback: () =&gt; void): void</SignatureBlock>

`describe(name, callback)` opens a suite and runs `callback` immediately, so the tests and hooks registered inside it belong to that suite. No test runs during registration: tests run after the program has registered everything, in the order they were written, and a nested suite runs where it appears.

A test's full name is its path through the suites, joined with ` > `, as in `discount > from 100 on > is ten percent`.

When an assertion fails while a suite's callback is registering, the suite is reported as failed and none of its tests run.

<!-- @include: ../../.fragments/modules/test/describe.md -->
