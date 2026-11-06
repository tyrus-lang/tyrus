---
title: Number theory
description: Number theory math functions exported by tyrus:math.
---

# Number theory

| Function | Signature | Description |
|----------|-----------|-------------|
| [`isPrime`](#isprime) `(n: u64) => boolean` | `true` if `n` is prime |
| [`nextPrime`](#nextprime) `(n: u64) => u64` | Smallest prime strictly greater than `n` |
| [`primeFactors`](#primefactors) `(n: u64) => u64[]` | Prime factors of `n` |
| [`fibonacci`](#fibonacci) `(n: u64) => u64` | `n`-th Fibonacci number |

#### `isPrime()`

Returns `true` if `n` is prime.

<SignatureBlock label="Signature">isPrime(n: u64): boolean</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/numbertheory-isPrime.md -->

#### `nextPrime()`

Returns the smallest prime strictly greater than `n`.

<SignatureBlock label="Signature">nextPrime(n: u64): u64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/numbertheory-nextPrime.md -->

#### `primeFactors()`

Returns the prime factors of `n`.

<SignatureBlock label="Signature">primeFactors(n: u64): u64[]</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/numbertheory-primeFactors.md -->

#### `fibonacci()`

Returns the `n`-th Fibonacci number.

<SignatureBlock label="Signature">fibonacci(n: u64): u64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/numbertheory-fibonacci.md -->
