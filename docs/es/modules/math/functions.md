---
title: Funciones
description: Funciones matemáticas exportadas por tyrus:math.
---

# Funciones

Todas las funciones de `tyrus:math` operan en `f64` salvo indicación contraria y se emiten como llamadas nativas directas.

## [Básicas y signos](./basics-and-signs.md)

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`abs`](./basics-and-signs.md#abs) | `(x: f64) => f64` | Valor absoluto |
| [`sign`](./basics-and-signs.md#sign) | `(x: f64) => f64` | Signo (`1`, `-1` o `0`) |
| [`sqrt`](./basics-and-signs.md#sqrt) | `(x: f64) => f64` | Raíz cuadrada |
| [`cbrt`](./basics-and-signs.md#cbrt) | `(x: f64) => f64` | Raíz cúbica |

## [Redondeo](./rounding.md)

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`floor`](./rounding.md#floor) | `(x: f64) => f64` | Redondea hacia abajo |
| [`ceil`](./rounding.md#ceil) | `(x: f64) => f64` | Redondea hacia arriba |
| [`trunc`](./rounding.md#trunc) | `(x: f64) => f64` | Trunca hacia cero |
| [`round`](./rounding.md#round) | `(x: f64) => f64` | Redondea al entero más cercano |
| [`roundTo`](./rounding.md#roundto) | `(x: f64, decimals: i32) => f64` | Redondea a N decimales |

## [Trigonometría](./trigonometry.md)

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`sin`](./trigonometry.md#sin) | `(x: f64) => f64` | Seno |
| [`cos`](./trigonometry.md#cos) | `(x: f64) => f64` | Coseno |
| [`tan`](./trigonometry.md#tan) | `(x: f64) => f64` | Tangente |
| [`asin`](./trigonometry.md#asin) | `(x: f64) => f64` | Arco seno |
| [`acos`](./trigonometry.md#acos) | `(x: f64) => f64` | Arco coseno |
| [`atan`](./trigonometry.md#atan) | `(x: f64) => f64` | Arco tangente |
| [`atan2`](./trigonometry.md#atan2) | `(y: f64, x: f64) => f64` | Arco tangente de `y/x` |
| [`degToRad`](./trigonometry.md#degtorad) | `(degrees: f64) => f64` | Convierte grados a radianes |
| [`radToDeg`](./trigonometry.md#radtodeg) | `(radians: f64) => f64` | Convierte radianes a grados |

## [Hiperbólicas](./hyperbolic.md)

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`sinh`](./hyperbolic.md#sinh) | `(x: f64) => f64` | Seno hiperbólico |
| [`cosh`](./hyperbolic.md#cosh) | `(x: f64) => f64` | Coseno hiperbólico |
| [`tanh`](./hyperbolic.md#tanh) | `(x: f64) => f64` | Tangente hiperbólica |

## [Exponenciales y logaritmos](./exponentials-and-logarithms.md)

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`exp`](./exponentials-and-logarithms.md#exp) | `(x: f64) => f64` | `e^x` |
| [`log`](./exponentials-and-logarithms.md#log) | `(x: f64) => f64` | Logaritmo natural |
| [`log10`](./exponentials-and-logarithms.md#log10) | `(x: f64) => f64` | Logaritmo base 10 |
| [`log2`](./exponentials-and-logarithms.md#log2) | `(x: f64) => f64` | Logaritmo base 2 |
| [`pow`](./exponentials-and-logarithms.md#pow) | `(base: f64, exp: f64) => f64` | Potencia |

## [Selección y bajo nivel](./selection-and-low-level.md)

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`max`](./selection-and-low-level.md#max) | `(a: f64, b: f64) => f64` | Máximo |
| [`min`](./selection-and-low-level.md#min) | `(a: f64, b: f64) => f64` | Mínimo |
| [`clamp`](./selection-and-low-level.md#clamp) | `(value: f64, min: f64, max: f64) => f64` | Limita valor al intervalo |
| [`clampMin`](./selection-and-low-level.md#clampmin) | `(value: f64, min: f64) => f64` | Garantiza valor mínimo |
| [`clampMax`](./selection-and-low-level.md#clampmax) | `(value: f64, max: f64) => f64` | Garantiza valor máximo |
| [`imul`](./selection-and-low-level.md#imul) | `(a: i32, b: i32) => i32` | Multiplicación C-style 32-bit |
| [`clz32`](./selection-and-low-level.md#clz32) | `(x: i32) => i32` | Cuenta bits cero a la izquierda en entero 32-bit |

## [Utilidades avanzadas](./advanced-utilities.md)

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`approxEqual`](./advanced-utilities.md#approxequal) | `(a: f64, b: f64, tolerance?: f64) => boolean` | Compara floats dentro de la tolerancia |
| [`lerp`](./advanced-utilities.md#lerp) | `(start: f64, end: f64, amount: f64) => f64` | Interpolación lineal |
| [`inverseLerp`](./advanced-utilities.md#inverselerp) | `(start: f64, end: f64, value: f64) => f64` | Interpolación lineal inversa |
| [`remap`](./advanced-utilities.md#remap) | `(value: f64, inMin: f64, inMax: f64, outMin: f64, outMax: f64) => f64` | Remapea valor entre escalas |
| [`smoothstep`](./advanced-utilities.md#smoothstep) | `(edge0: f64, edge1: f64, x: f64) => f64` | Interpolación suave de Hermite |
| [`gcd`](./advanced-utilities.md#gcd) | `(a: u32, b: u32) => u32` | Máximo divisor común |
| [`lcm`](./advanced-utilities.md#lcm) | `(a: u32, b: u32) => u32` | Mínimo múltiplo común |
| [`randomRange`](./advanced-utilities.md#randomrange) | `(min: f64, max: f64) => f64` | Float aleatorio en el intervalo semiabierto |

## [Estadística y combinatoria](./statistics-and-combinatorics.md)

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`mean`](./statistics-and-combinatorics.md#mean) | `(values: f64[]) => f64` | Media aritmética; `0.0` para array vacía |
| [`median`](./statistics-and-combinatorics.md#median) | `(values: f64[]) => f64` | Mediana; `0.0` para array vacía |
| [`mode`](./statistics-and-combinatorics.md#mode) | `(values: f64[]) => f64` | Valor más frecuente; `0.0` para array vacía |
| [`variance`](./statistics-and-combinatorics.md#variance) | `(values: f64[]) => f64` | Varianza poblacional; `0.0` para array vacía |
| [`stdDev`](./statistics-and-combinatorics.md#stddev) | `(values: f64[]) => f64` | Desviación estándar poblacional; `0.0` para array vacía |
| [`percentile`](./statistics-and-combinatorics.md#percentile) | `(p: f64, values: f64[]) => f64` | Percentil `p` (0-100) con interpolación lineal; `0.0` para array vacía |
| [`factorial`](./statistics-and-combinatorics.md#factorial) | `(n: u128) => u128` | `n!` |
| [`permutation`](./statistics-and-combinatorics.md#permutation) | `(n: u128, k: u128) => u128` | `n! / (n - k)!` |
| [`combination`](./statistics-and-combinatorics.md#combination) | `(n: u128, k: u128) => u128` | `n! / (k! * (n - k)!)` |

## [Teoría de números](./number-theory.md)

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`isPrime`](./number-theory.md#isprime) | `(n: u64) => boolean` | `true` si `n` es primo |
| [`nextPrime`](./number-theory.md#nextprime) | `(n: u64) => u64` | Menor primo estrictamente mayor que `n` |
| [`primeFactors`](./number-theory.md#primefactors) | `(n: u64) => u64[]` | Factores primos de `n` |
| [`fibonacci`](./number-theory.md#fibonacci) | `(n: u64) => u64` | `n`-ésimo número de Fibonacci |

## [Geometría y utilidades numéricas](./geometry-and-numeric-utilities.md)

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`hypot`](./geometry-and-numeric-utilities.md#hypot) | `(x: f64, y: f64) => f64` | Norma Euclidiana `sqrt(x^2 + y^2)` |
| [`toPolar`](./geometry-and-numeric-utilities.md#topolar) | `(x: f64, y: f64) => [f64, f64]` | Convierte Cartesianas a polares `(r, theta)` |
| [`toCartesian`](./geometry-and-numeric-utilities.md#tocartesian) | `(r: f64, theta: f64) => [f64, f64]` | Convierte polares a Cartesianas `(x, y)` |
| [`copysign`](./geometry-and-numeric-utilities.md#copysign) | `(x: f64, y: f64) => f64` | Copia signo de `y` a `x` |
| [`fmod`](./geometry-and-numeric-utilities.md#fmod) | `(x: f64, y: f64) => f64` | Resto en punto flotante de `x / y` |
| [`logBase`](./geometry-and-numeric-utilities.md#logbase) | `(x: f64, base: f64) => f64` | Logaritmo de `x` en la `base` dada |
| [`powi`](./geometry-and-numeric-utilities.md#powi) | `(x: f64, exp: i32) => f64` | `x` elevado a la potencia entera `exp` |
