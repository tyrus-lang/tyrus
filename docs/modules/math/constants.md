---
title: Constants
description: Mathematical constants exported by tyrus:math.
---

# Constants

All `tyrus:math` constants are `f64` values resolved at compile time.

<!-- @include: ../../.fragments/modules/math/constants/constants.md -->

| Constant | Value | Description |
|----------|-------|-------------|
| `PI` | `3.141592653589793` | Ratio of circumference to diameter |
| `E` | `2.718281828459045` | Base of natural logarithms |
| `LN2` | `0.6931471805599453` | Natural logarithm of 2 |
| `LN10` | `2.302585092994046` | Natural logarithm of 10 |
| `LOG2E` | `1.4426950408889634` | Base-2 logarithm of E |
| `LOG10E` | `0.4342944819032518` | Base-10 logarithm of E |
| `SQRT2` | `1.4142135623730951` | Square root of 2 |
| `EPSILON` | `2.220446049250313e-16` | Gap from 1 to the next `f64` |
| `TAU` | `6.283185307179586` | Two times PI |
| `PHI` | `1.618033988749895` | Golden ratio |

::: tip Zero-cost constants
These constants are emitted as initialized globals in the generated binary. Importing them does not add runtime overhead beyond reading a global value.
:::
