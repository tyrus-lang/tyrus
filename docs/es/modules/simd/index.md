---
title: tyrus:simd
description: Operaciones vectorizadas explícitas sobre arrays numéricos.
---

# tyrus:simd

`tyrus:simd` ofrece operaciones vectorizadas explícitas sobre arrays numéricos. Consulta [`simdAdd`](./simd-add.md), [`simdSub`](./simd-sub.md), [`simdMul`](./simd-mul.md), [`simdDiv`](./simd-div.md) y [`SimdError`](./simd-error.md).

`tyrus:simd` acepta directamente arrays dinámicos (`T[]`) y arrays de tamaño fijo, como `[i32; 5]`. Los arrays fijos usan el tamaño conocido en compilación y no necesitan una conversión en un binding.
