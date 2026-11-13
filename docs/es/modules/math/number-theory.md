---
title: Teoría de números
description: Funciones de teoría de números exportadas por tyrus:math.
---

# Teoría de números

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`isPrime`](#isprime) | `(n: u64) => boolean` | `true` si `n` es primo |
| [`nextPrime`](#nextprime) | `(n: u64) => u64` | Menor primo estrictamente mayor que `n` |
| [`primeFactors`](#primefactors) | `(n: u64) => u64[]` | Factores primos de `n` |
| [`fibonacci`](#fibonacci) | `(n: u64) => u64` | `n`-ésimo número de Fibonacci |

#### `isPrime()`

Devuelve `true` si `n` es primo.

<SignatureBlock label="Firma">isPrime(n: u64): boolean</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/numbertheory-isPrime.md -->

#### `nextPrime()`

Devuelve el menor primo estrictamente mayor que `n`.

<SignatureBlock label="Firma">nextPrime(n: u64): u64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/numbertheory-nextPrime.md -->

#### `primeFactors()`

Devuelve los factores primos de `n`.

<SignatureBlock label="Firma">primeFactors(n: u64): u64[]</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/numbertheory-primeFactors.md -->

#### `fibonacci()`

Devuelve el `n`-ésimo número de Fibonacci.

<SignatureBlock label="Firma">fibonacci(n: u64): u64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/numbertheory-fibonacci.md -->
