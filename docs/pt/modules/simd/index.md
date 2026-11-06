---
title: tyrus:simd
description: Operações vetorizadas explícitas sobre arrays numéricos.
---

# tyrus:simd

`tyrus:simd` oferece operações vetorizadas explícitas sobre arrays numéricos. Consulte [`simdAdd`](./simd-add.md), [`simdSub`](./simd-sub.md), [`simdMul`](./simd-mul.md), [`simdDiv`](./simd-div.md) e [`SimdError`](./simd-error.md).

`tyrus:simd` aceita diretamente arrays dinâmicos (`T[]`) e arrays de tamanho fixo, como `[i32; 5]`. Arrays fixos usam o tamanho conhecido em compilação e não precisam de conversão em um binding.
