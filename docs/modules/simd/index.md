---
title: tyrus:simd
description: Explicit vectorized operations over numeric arrays.
---

# tyrus:simd

`tyrus:simd` provides explicit vectorized operations over numeric arrays. See [`simdAdd`](./simd-add.md), [`simdSub`](./simd-sub.md), [`simdMul`](./simd-mul.md), [`simdDiv`](./simd-div.md), and [`SimdError`](./simd-error.md).

`tyrus:simd` accepts dynamic arrays (`T[]`) and fixed-size arrays such as `[i32; 5]` directly. Fixed-size inputs use their compile-time length and do not need a conversion binding.
