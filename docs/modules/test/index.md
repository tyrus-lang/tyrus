---
title: Test
description: Test suites, assertions, hooks, and modifiers run by `tyrus test`.
---

# Test

`tyrus:test` is the test framework included with the language. Suites are written with `describe`, tests with `test` or its alias `it`, and checks with `expect`. `tyrus test` discovers the test files, compiles each one into its own binary, and reports every result.

<!-- @include: ../../.fragments/modules/test/describe.md -->

Every name is imported explicitly, as with any other native module. A test file is any file whose name ends in `.test.tyr`; `__tests__/` is the conventional folder for them, but not a required one.

## API

- [describe](./describe): group tests into a named suite; suites can nest.
- [test / it](./test): register a named test.
- [expect](./expect): check a value with a matcher, inverted with `.not`.
- [Hooks](./hooks): `beforeAll`, `afterAll`, `beforeEach` and `afterEach`.
- [Modifiers](./modifiers): `.skip`, `.only` and `.todo` on tests and suites.
- [Running Tests](./running): discovery, command-line options, and the report.
