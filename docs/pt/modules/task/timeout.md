---
title: timeout
description: Limita quanto tempo uma Promise pode levar, rejeitando e cancelando-a quando o prazo se esgota.
---

# timeout

`timeout` corre `task` contra um prazo de `ms` milissegundos.

<SignatureBlock label="Assinatura">timeout&lt;T&gt;(ms: u64, task: Promise&lt;T&gt;): Promise&lt;T&gt;</SignatureBlock>

Se `task` se estabelece antes do prazo, `timeout` adota seu resultado, resolvendo ou rejeitando do mesmo jeito que `task` fez. Se o prazo se esgota primeiro, `timeout` rejeita e cancela `task` cooperativamente através do mesmo mecanismo que `cancel` usa, da mesma forma que `race` cancela os handles que não escolheu.

<!-- @include: ../../../.fragments/modules/task/timeout.md -->
