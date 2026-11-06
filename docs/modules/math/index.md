---
title: Math
description: Mathematical constants and functions from tyrus:math.
---

# Math

`tyrus:math` provides mathematical constants and functions. All constants are `f64` values available at compile time with zero runtime cost.

## Imports

<SignatureBlock label="Imports">import { PI, E, TAU } from "tyrus:math";</SignatureBlock>

## Exported Items

| Item | Type | Purpose |
|------|------|---------|
| [`Constants`](./constants.md) | `f64` constants | Fundamental mathematical constants |
| [`Functions`](./functions.md) | native functions | Basic, trigonometric, hyperbolic, exponential, logarithmic, and selection helpers |

## Quick Start

A minimal example using mathematical constants:

<!-- @include: ../../.fragments/modules/math/index/quick-start.md -->

::: tip Zero-cost constants
These constants are emitted as initialized globals in the generated binary. Importing them does not add runtime overhead beyond reading a global value.
:::
