---
title: race
description: Resolve ou rejeita para o JoinHandle de um array que se estabelecer primeiro, cancelando o resto.
---

# race

`race` resolve ou rejeita para o `JoinHandle<T>` de um array que se estabelecer primeiro.

<SignatureBlock label="Assinatura">race&lt;T&gt;(handles: JoinHandle&lt;T&gt;[]): Promise&lt;T&gt;</SignatureBlock>

Assim que um handle se estabelece, `race` cancela cooperativamente todo o resto através do mesmo mecanismo de `cancel`/`JoinHandle<T>.abort()`, um handle que já se estabeleceu até então (o próprio vencedor, ou um perdedor que por acaso terminou entre a corrida se resolver e essa limpeza rodar) é deixado quieto, já que cancelar uma Promise já estabelecida é um no-op inofensivo. Todo handle passado para `race` é consumido do mesmo jeito que `.join()` consome um, reusar um handle já passado para `race`, para `joinAll`, ou para seu próprio `.join()`, é rejeitado em tempo de compilação (`E3033`).

<!-- @include: ../../../.fragments/modules/task/race.md -->
