---
title: isCancelled
description: Pergunte, de dentro de uma tarefa, se ela foi mandada parar.
---

# isCancelled

`isCancelled` responde se a tarefa em execução foi mandada parar.

<SignatureBlock label="Assinatura">isCancelled(): boolean</SignatureBlock>

Só pode ser usado dentro de uma função `async`, porque a resposta é sobre a tarefa que está rodando.

O cancelamento é cooperativo: ele tem efeito num ponto de suspensão. Uma tarefa que passa muito tempo computando não alcança nenhum, e por isso nunca é interrompida. Isso é consequência do desenho e não um buraco nele, e é o que um laço desses usa para parar sozinho.

<!-- @include: ../../../.fragments/modules/task/is-cancelled.md -->

Parar assim continua sendo um cancelamento. Um valor que a tarefa retorne depois de ver o pedido não vira o valor da Promise: trabalho que já foi mandado parar não chega a reportar resultado.
