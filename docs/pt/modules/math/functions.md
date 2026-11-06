---
title: Funções
description: Funções matemáticas exportadas por tyrus:math.
---

# Funções

Todas as funções de `tyrus:math` operam em `f64` salvo indicação contrária e são emitidas como chamadas nativas diretas.

## [Básicas e sinais](./basics-and-signs.md)

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`abs`](./basics-and-signs.md#abs) `(x: f64) => f64` | Valor absoluto |
| [`sign`](./basics-and-signs.md#sign) `(x: f64) => f64` | Sinal (`1`, `-1` ou `0`) |
| [`sqrt`](./basics-and-signs.md#sqrt) `(x: f64) => f64` | Raiz quadrada |
| [`cbrt`](./basics-and-signs.md#cbrt) `(x: f64) => f64` | Raiz cúbica |

## [Arredondamento](./rounding.md)

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`floor`](./rounding.md#floor) `(x: f64) => f64` | Arredonda para baixo |
| [`ceil`](./rounding.md#ceil) `(x: f64) => f64` | Arredonda para cima |
| [`trunc`](./rounding.md#trunc) `(x: f64) => f64` | Trunca em direção a zero |
| [`round`](./rounding.md#round) `(x: f64) => f64` | Arredonda para o inteiro mais próximo |
| [`roundTo`](./rounding.md#roundto) `(x: f64, decimals: i32) => f64` | Arredonda para N casas decimais |

## [Trigonometria](./trigonometry.md)

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`sin`](./trigonometry.md#sin) `(x: f64) => f64` | Seno |
| [`cos`](./trigonometry.md#cos) `(x: f64) => f64` | Cosseno |
| [`tan`](./trigonometry.md#tan) `(x: f64) => f64` | Tangente |
| [`asin`](./trigonometry.md#asin) `(x: f64) => f64` | Arco seno |
| [`acos`](./trigonometry.md#acos) `(x: f64) => f64` | Arco cosseno |
| [`atan`](./trigonometry.md#atan) `(x: f64) => f64` | Arco tangente |
| [`atan2`](./trigonometry.md#atan2) `(y: f64, x: f64) => f64` | Arco tangente de `y/x` |
| [`degToRad`](./trigonometry.md#degtorad) `(degrees: f64) => f64` | Converte graus para radianos |
| [`radToDeg`](./trigonometry.md#radtodeg) `(radians: f64) => f64` | Converte radianos para graus |

## [Hiperbólicas](./hyperbolic.md)

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`sinh`](./hyperbolic.md#sinh) `(x: f64) => f64` | Seno hiperbólico |
| [`cosh`](./hyperbolic.md#cosh) `(x: f64) => f64` | Cosseno hiperbólico |
| [`tanh`](./hyperbolic.md#tanh) `(x: f64) => f64` | Tangente hiperbólica |

## [Exponenciais e logaritmos](./exponentials-and-logarithms.md)

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`exp`](./exponentials-and-logarithms.md#exp) `(x: f64) => f64` | `e^x` |
| [`log`](./exponentials-and-logarithms.md#log) `(x: f64) => f64` | Logaritmo natural |
| [`log10`](./exponentials-and-logarithms.md#log10) `(x: f64) => f64` | Logaritmo base 10 |
| [`log2`](./exponentials-and-logarithms.md#log2) `(x: f64) => f64` | Logaritmo base 2 |
| [`pow`](./exponentials-and-logarithms.md#pow) `(base: f64, exp: f64) => f64` | Potência |

## [Seleção e baixo nível](./selection-and-low-level.md)

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`max`](./selection-and-low-level.md#max) `(a: f64, b: f64) => f64` | Máximo |
| [`min`](./selection-and-low-level.md#min) `(a: f64, b: f64) => f64` | Mínimo |
| [`clamp`](./selection-and-low-level.md#clamp) `(value: f64, min: f64, max: f64) => f64` | Limita valor ao intervalo |
| [`clampMin`](./selection-and-low-level.md#clampmin) `(value: f64, min: f64) => f64` | Garante valor mínimo |
| [`clampMax`](./selection-and-low-level.md#clampmax) `(value: f64, max: f64) => f64` | Garante valor máximo |
| [`imul`](./selection-and-low-level.md#imul) `(a: i32, b: i32) => i32` | Multiplicação C-style 32-bit |
| [`clz32`](./selection-and-low-level.md#clz32) `(x: i32) => i32` | Conta bits zero à esquerda em inteiro 32-bit |

## [Utilitários avançados](./advanced-utilities.md)

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`approxEqual`](./advanced-utilities.md#approxequal) `(a: f64, b: f64, tolerance?: f64) => boolean` | Compara floats dentro da tolerância |
| [`lerp`](./advanced-utilities.md#lerp) `(start: f64, end: f64, amount: f64) => f64` | Interpolação linear |
| [`inverseLerp`](./advanced-utilities.md#inverselerp) `(start: f64, end: f64, value: f64) => f64` | Interpolação linear inversa |
| [`remap`](./advanced-utilities.md#remap) `(value: f64, inMin: f64, inMax: f64, outMin: f64, outMax: f64) => f64` | Remapeia valor entre escalas |
| [`smoothstep`](./advanced-utilities.md#smoothstep) `(edge0: f64, edge1: f64, x: f64) => f64` | Interpolação suave de Hermite |
| [`gcd`](./advanced-utilities.md#gcd) `(a: u32, b: u32) => u32` | Máximo divisor comum |
| [`lcm`](./advanced-utilities.md#lcm) `(a: u32, b: u32) => u32` | Mínimo múltiplo comum |
| [`randomRange`](./advanced-utilities.md#randomrange) `(min: f64, max: f64) => f64` | Float aleatório no intervalo semiaberto |

## [Estatística e combinatória](./statistics-and-combinatorics.md)

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`mean`](./statistics-and-combinatorics.md#mean) `(values: f64[]) => f64` | Média aritmética; `0.0` para array vazia |
| [`median`](./statistics-and-combinatorics.md#median) `(values: f64[]) => f64` | Mediana; `0.0` para array vazia |
| [`mode`](./statistics-and-combinatorics.md#mode) `(values: f64[]) => f64` | Valor mais frequente; `0.0` para array vazia |
| [`variance`](./statistics-and-combinatorics.md#variance) `(values: f64[]) => f64` | Variância populacional; `0.0` para array vazia |
| [`stdDev`](./statistics-and-combinatorics.md#stddev) `(values: f64[]) => f64` | Desvio padrão populacional; `0.0` para array vazia |
| [`percentile`](./statistics-and-combinatorics.md#percentile) `(p: f64, values: f64[]) => f64` | Percentil `p` (0-100) com interpolação linear; `0.0` para array vazia |
| [`factorial`](./statistics-and-combinatorics.md#factorial) `(n: u128) => u128` | `n!` |
| [`permutation`](./statistics-and-combinatorics.md#permutation) `(n: u128, k: u128) => u128` | `n! / (n - k)!` |
| [`combination`](./statistics-and-combinatorics.md#combination) `(n: u128, k: u128) => u128` | `n! / (k! * (n - k)!)` |

## [Teoria dos números](./number-theory.md)

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`isPrime`](./number-theory.md#isprime) `(n: u64) => boolean` | `true` se `n` é primo |
| [`nextPrime`](./number-theory.md#nextprime) `(n: u64) => u64` | Menor primo estritamente maior que `n` |
| [`primeFactors`](./number-theory.md#primefactors) `(n: u64) => u64[]` | Fatores primos de `n` |
| [`fibonacci`](./number-theory.md#fibonacci) `(n: u64) => u64` | `n`-ésimo número de Fibonacci |

## [Geometria e utilitários numéricos](./geometry-and-numeric-utilities.md)

| Função | Assinatura | Descrição |
|--------|-----------|-----------|
| [`hypot`](./geometry-and-numeric-utilities.md#hypot) `(x: f64, y: f64) => f64` | Norma Euclidiana `sqrt(x^2 + y^2)` |
| [`toPolar`](./geometry-and-numeric-utilities.md#topolar) `(x: f64, y: f64) => [f64, f64]` | Converte Cartesianas para polares `(r, theta)` |
| [`toCartesian`](./geometry-and-numeric-utilities.md#tocartesian) `(r: f64, theta: f64) => [f64, f64]` | Converte polares para Cartesianas `(x, y)` |
| [`copysign`](./geometry-and-numeric-utilities.md#copysign) `(x: f64, y: f64) => f64` | Copia sinal de `y` para `x` |
| [`fmod`](./geometry-and-numeric-utilities.md#fmod) `(x: f64, y: f64) => f64` | Resto floating-point de `x / y` |
| [`logBase`](./geometry-and-numeric-utilities.md#logbase) `(x: f64, base: f64) => f64` | Logaritmo de `x` na `base` dada |
| [`powi`](./geometry-and-numeric-utilities.md#powi) `(x: f64, exp: i32) => f64` | `x` elevado à potência inteira `exp` |
