---
title: timeout
description: Acota cuánto puede tardar una Promise, rechazándola y cancelándola cuando el plazo se agota.
---

# timeout

`timeout` corre `task` contra un plazo de `ms` milisegundos.

<SignatureBlock label="Firma">timeout&lt;T&gt;(ms: u64, task: Promise&lt;T&gt;): Promise&lt;T&gt;</SignatureBlock>

Si `task` se asienta antes del plazo, `timeout` adopta su resultado, resolviéndose o rechazándose de la misma forma que `task` lo hizo. Si el plazo se agota primero, `timeout` se rechaza y cancela `task` cooperativamente a través del mismo mecanismo que usa `cancel`, de la misma forma que `race` cancela los handles que no eligió.

<!-- @include: ../../../.fragments/modules/task/timeout.md -->
