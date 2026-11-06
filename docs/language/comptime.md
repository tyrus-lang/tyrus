---
title: Compile-time Evaluation
description: Evaluate deterministic expressions and build constants during compilation with comptime.
---

# Compile-time Evaluation

`comptime(expression)` evaluates a deterministic expression while compiling and replaces the wrapper with an ordinary literal in the generated program. It can evaluate literals directly or call an explicitly eligible `comptime function` or `comptime const` arrow. The same qualified function remains callable normally at runtime.

## Precompute Fibonacci

This example calculates the Fibonacci number at position 50 during compilation. The iterative implementation completes in 50 steps, while a naive recursive version would repeat work exponentially and could exceed the deterministic evaluation budget. The result uses `i64` because it does not fit in `i32`.

<!-- @include: ../.fragments/language/comptime/fibonacci.md -->

The loop and intermediate mutations run only in the compiler. At runtime, `fibonacci50` is already the literal `12586269025`. Calling `fibonacci(...)` outside `comptime(...)` remains valid and executes normally at runtime.

## Eligible Functions

A compile-time call target must use `comptime function name(...)` or `comptime const name = (...) => ...`. Eligible code may use local bindings, conditionals, loops, recursion, `break`, `continue`, arrays, tuples, records, enums and deterministic arithmetic. It cannot read mutable external state, call an ordinary or native function, perform I/O, use time or randomness, throw or catch, use `async`/`await`, or create a value with runtime identity. A compile-time arrow may capture only constants that can themselves be evaluated during compilation.

## Values and Limits

Compile-time results may be booleans, integers, supported floating-point values, decimals, chars, strings, `void`, arrays, tuples, records and enums whose contents are also compile-time values. Closures, function values, references, promises, regexes, `Map`, `Set` and native resources are rejected. Each wrapper has deterministic limits of 1,000,000 evaluation steps, 256 nested calls, 65,536 structural elements and 16 MiB of temporary and materialized values.

## Editor Feedback

Hover over the `comptime` keyword to see the evaluated type and a bounded summary of its value. Failures point at their exact cause and include the wrapper and call trace as related information. When an expression is valid at runtime but not eligible for compile-time evaluation, the editor can remove the wrapper with the `Evaluate this expression at runtime` Quick Fix.
