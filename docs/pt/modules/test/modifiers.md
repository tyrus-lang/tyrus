---
title: Modificadores
description: Ignora, foca e planeja testes.
---

# Modificadores

<SignatureBlock label="Assinatura">test.skip(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Assinatura">test.only(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Assinatura">test.todo(name: string): void</SignatureBlock>

<SignatureBlock label="Assinatura">describe.skip(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Assinatura">describe.only(name: string, callback: () =&gt; void): void</SignatureBlock>

`test.skip` lista um teste como ignorado sem executá-lo; o seu callback ainda precisa compilar. `describe.skip` ignora todos os testes da suíte, e os hooks da suíte não executam.

`test.todo(name)` lista um teste que ainda não foi escrito. Testes ignorados e todos são contados separadamente no resumo.

`it` aceita os mesmos modificadores, assim como um import com alias, por exemplo `import { test as check } from "tyrus:test"`. `test.concurrent` e `test.fails` ainda não estão implementados e são reportados como erros de compilação.

<!-- @include: ../../../.fragments/modules/test/modifiers.md -->

## Foco com `.only`

Quando algum teste ou suíte de um programa está focado com `.only`, só os testes focados executam e os demais contam como ignorados. `.skip` prevalece sobre `.only`: focar nunca executa um teste que foi ignorado de propósito.

<!-- @include: ../../../.fragments/modules/test/modifiers-only.md -->

## Um teste por caso

`test.each(name, cases, callback)` registra um teste por caso, com os nomes `name [0]`, `name [1]` e assim por diante. O callback recebe um caso; escrito inline, o tipo do parâmetro é inferido a partir dos casos. Os casos precisam ser um literal de array, porque são desdobrados quando o programa é compilado.

<!-- @include: ../../../.fragments/modules/test/each.md -->
