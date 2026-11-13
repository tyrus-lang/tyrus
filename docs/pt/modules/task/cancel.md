---
title: cancel
description: Peça a uma tarefa que pare, e espere ela terminar de parar.
---

# cancel

`cancel` pede a uma Promise pendente que pare. O trabalho desenrola pela limpeza dele e a Promise settla como cancelada.

<SignatureBlock label="Assinatura">cancel&lt;T&gt;(promise: Promise&lt;T&gt;, reason: string, graceMillis?: u64): boolean</SignatureBlock>

Devolve `true` apenas para a chamada que inicia o cancelamento, e `false` para uma que chega depois de a Promise já ter settlado ou já ter sido mandada parar.

O cancelamento é cooperativo: ele tem efeito num ponto de suspensão, nunca no meio de um statement. Uma tarefa no meio de uma computação longa não alcança ponto de suspensão nenhum e por isso não é interrompida; [`isCancelled`](./is-cancelled.md) é o que um laço desses usa para parar sozinho.

## O que roda, e o que não

Uma tarefa cancelada roda os blocos `finally` dela e então sai. Tudo o que ela segura atravessando um `await` é liberado, e os statements depois do bloco não rodam.

Um `catch` não enxerga o cancelamento. Um `catch` é escrito para as falhas que o programa espera do próprio trabalho, e tratar "você foi mandado parar" como uma delas significa que um handler que registra e segue, ou que tenta de novo, começa trabalho novo numa tarefa que já está terminando.

Ser cancelado e esperar por algo cancelado são coisas diferentes, e só a primeira pula o `catch`. Uma tarefa que espera por uma Promise que outra pessoa cancelou não foi mandada parar, e o `catch` dela é o único lugar onde ela pode saber o que aconteceu.

<!-- @include: ../../../.fragments/modules/task/cancel.md -->

## Quando a Promise settla

Quando o trabalho termina de desenrolar, não quando o pedido é feito. Um chamador que para um worker e em seguida reusa a porta, o arquivo ou a conexão dele estaria correndo contra a liberação, o que aparece como uma falha rara em vez de uma resposta errada.

O motivo que o chamador passou é o que a Promise carrega, seja lá o que a tarefa alcançou na saída: um valor que ela conseguiu retornar é produto de trabalho que já tinha sido mandado parar.

O cancelamento alcança o trabalho que o runtime faz em nome da tarefa. Uma tarefa esperando por uma requisição tem essa requisição interrompida, e a conexão fecha.

## O prazo

`graceMillis` limita quanto tempo a limpeza pode levar. Passado ele, o trabalho é cortado, e a Promise carrega `cancellation cleanup exceeded its deadline` em vez do motivo original, porque de outro modo o chamador não teria como distinguir uma parada limpa de uma que abandonou o trabalho pela metade.

Omiti-lo significa que a limpeza nunca é cortada. Limpeza que ainda está progredindo não é algo para interromper por suposição, então quem tem pressa é quem diz quanto vai esperar.
