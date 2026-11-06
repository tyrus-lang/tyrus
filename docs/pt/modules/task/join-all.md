---
title: joinAll
description: Espera cada JoinHandle de um array se estabelecer, mantendo os resultados na ordem.
---

# joinAll

`joinAll` espera cada `JoinHandle<T>` de um array se estabelecer, resolvendo para seus resultados na mesma ordem em que os handles foram passados.

<SignatureBlock label="Assinatura">joinAll&lt;T&gt;(handles: JoinHandle&lt;T&gt;[]): Promise&lt;T[]&gt;</SignatureBlock>

Resolve assim que todo handle já resolveu, não importa qual terminou primeiro de verdade, `results[0]` é sempre o resultado de `handles[0]`, não o que se estabeleceu mais cedo. Rejeita com a primeira rejeição que observa, da mesma forma que `Promise.all` faz. Todo handle passado para `joinAll` é consumido do mesmo jeito que `.join()` consome um, reusar um handle já passado para `joinAll`, para `race`, ou para seu próprio `.join()`, é rejeitado em tempo de compilação (`E3033`).

<!-- @include: ../../../.fragments/modules/task/join-all.md -->
