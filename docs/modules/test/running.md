---
title: Running Tests
description: Discovery and the report.
---

# Running Tests

## Discovery

A test file is any file whose name ends in `.test.tyr`, wherever it is in the project. `__tests__/` is the conventional folder for them, but not a required one: a test can also sit next to the code it covers. `tyrus test` finds every test file, skipping `target/` and hidden directories, compiles each one into its own binary and runs it in its own process, so a file that crashes does not stop the others, and a run with more than one file ends with a combined summary. Pass a file to run only that file, or a directory to run the test files under it.

Only a test file may import `tyrus:test`. A module of shared test helpers that calls `expect` is therefore a test file too: name it `helpers.test.tyr`, import it as `#/helpers.test`, and it contributes no tests of its own. The import is rejected anywhere else (`E0434`), including in a helper module a test imports, so the runner the module arms never reaches a production binary.

Test files compile and run in parallel, as many at once as there are processors. Output is grouped per file and printed in discovery order however the processes finished, and color follows the terminal and `NO_COLOR`.

<!-- @include: ../../.fragments/tooling/cli/test-options.md -->

## Report

Each file prints one line per test, indented under its suites, followed by a summary. A failed test prints its message under its name, and the process exits with status 1 when any test fails.

<!-- @include: ../../.fragments/tooling/cli/test-report.md -->
