---
title: Teoria dos números
description: Funções de teoria dos números exportadas por tyrus:math.
---

# Teoria dos números

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`isPrime`](#isprime) | `(n: u64) => boolean` | `true` se `n` é primo |
| [`nextPrime`](#nextprime) | `(n: u64) => u64` | Menor primo estritamente maior que `n` |
| [`primeFactors`](#primefactors) | `(n: u64) => u64[]` | Fatores primos de `n` |
| [`fibonacci`](#fibonacci) | `(n: u64) => u64` | `n`-ésimo número de Fibonacci |

#### `isPrime()`

Retorna `true` se `n` é primo.

<SignatureBlock label="Assinatura">isPrime(n: u64): boolean</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/numbertheory-isPrime.md -->

#### `nextPrime()`

Retorna o menor primo estritamente maior que `n`.

<SignatureBlock label="Assinatura">nextPrime(n: u64): u64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/numbertheory-nextPrime.md -->

#### `primeFactors()`

Retorna os fatores primos de `n`.

<SignatureBlock label="Assinatura">primeFactors(n: u64): u64[]</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/numbertheory-primeFactors.md -->

#### `fibonacci()`

Retorna o `n`-ésimo número de Fibonacci.

<SignatureBlock label="Assinatura">fibonacci(n: u64): u64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/numbertheory-fibonacci.md -->
