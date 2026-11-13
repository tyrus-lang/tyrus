---
title: Hooks
description: Executa preparação e limpeza em torno dos testes.
---

# Hooks

<SignatureBlock label="Assinatura">beforeAll(callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Assinatura">afterAll(callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Assinatura">beforeEach(callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Assinatura">afterEach(callback: () =&gt; void): void</SignatureBlock>

`beforeAll` executa uma vez antes do primeiro teste da sua suíte e `afterAll` executa uma vez depois do último. `beforeEach` e `afterEach` executam em torno de cada teste da sua suíte e de todas as suítes aninhadas: `beforeEach` da suíte mais externa para dentro, e `afterEach` na ordem inversa. Hooks registrados fora de qualquer `describe` valem para o arquivo inteiro.

`afterEach` executa mesmo quando o teste falhou, então os recursos adquiridos na preparação são liberados. Uma suíte sem nenhum teste para executar, por causa de `.skip` ou `.only`, não executa seus hooks.

Um `beforeAll` que falha reprova todos os testes da sua suíte sem executá-los. Um `beforeEach` que falha reprova o seu teste, e um `afterEach` que falha é acrescentado ao relatório do teste depois do qual executou.

Hooks também podem ser `async`; cada um é aguardado antes do teste que prepara.

<!-- @include: ../../../.fragments/modules/test/hooks.md -->
