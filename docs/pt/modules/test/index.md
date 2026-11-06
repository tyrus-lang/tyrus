---
title: Test
description: Suítes de teste, asserções, hooks e modificadores executados por `tyrus test`.
---

# Test

`tyrus:test` é o framework de testes incluído na linguagem. Suítes são escritas com `describe`, testes com `test` ou seu alias `it`, e verificações com `expect`. `tyrus test` descobre os arquivos de teste, compila cada um em um binário próprio e reporta todos os resultados.

<!-- @include: ../../../.fragments/modules/test/describe.md -->

Todo nome é importado explicitamente, como em qualquer outro módulo nativo. Um arquivo de teste é qualquer arquivo cujo nome termina em `.test.tyr`; `__tests__/` é a pasta convencional para eles, mas não obrigatória.

## API

- [describe](./describe): agrupa testes em uma suíte nomeada; suítes podem ser aninhadas.
- [test / it](./test): registra um teste nomeado.
- [expect](./expect): verifica um valor com um matcher, invertido com `.not`.
- [Hooks](./hooks): `beforeAll`, `afterAll`, `beforeEach` e `afterEach`.
- [Modificadores](./modifiers): `.skip`, `.only` e `.todo` em testes e suítes.
- [Executando testes](./running): descoberta, opções de linha de comando e o relatório.
