---
title: test / it
description: Registra um teste nomeado.
---

# test / it

<SignatureBlock label="Assinatura">test(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Assinatura">it(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Assinatura">test(name: string, callback: () =&gt; Promise&lt;void&gt;): void</SignatureBlock>

`test` e `it` são a mesma função com dois nomes. O callback executa uma vez. O teste passa quando o callback retorna e falha quando uma asserção falha ou o callback lança; a mensagem de falha é impressa abaixo do nome do teste.

Um callback não pode atribuir a um binding declarado fora dele, porque o callback sobrevive ao escopo que declarou o binding (`E0436`). Mantenha a preparação de que um teste precisa dentro desse teste, ou em um `beforeEach` que prepare um estado que o teste lê por meio de uma função.

<!-- @include: ../../../.fragments/modules/test/test.md -->

## Testes assíncronos

Um callback `async` é aguardado antes que o próximo hook ou teste execute. Uma asserção que falha depois de um `await` reprova o teste como qualquer outra, assim como uma promise rejeitada.

<!-- @include: ../../../.fragments/modules/test/test-async.md -->
