---
title: Executando testes
description: Descoberta e o relatório.
---

# Executando testes

## Descoberta

Um arquivo de teste é qualquer arquivo cujo nome termina em `.test.tyr`, onde quer que esteja no projeto. `__tests__/` é a pasta convencional para eles, mas não obrigatória: um teste também pode ficar ao lado do código que cobre. `tyrus test` encontra todos os arquivos de teste, ignorando `target/` e diretórios ocultos, compila cada um em um binário próprio e o executa em um processo próprio, então um arquivo que cai não interrompe os outros, e uma execução com mais de um arquivo termina com um resumo combinado. Passe um arquivo para executar só esse arquivo, ou um diretório para executar os arquivos de teste dentro dele.

Só um arquivo de teste pode importar `tyrus:test`. Um módulo de helpers de teste compartilhados que chama `expect` é, portanto, também um arquivo de teste: chame-o de `helpers.test.tyr`, importe-o como `#/helpers.test`, e ele não contribui com testes próprios. O import é recusado em qualquer outro lugar (`E0434`), inclusive em um módulo auxiliar que um teste importe, então o runner que o módulo arma nunca chega a um binário de produção.

Os arquivos de teste são compilados e executados em paralelo, tantos ao mesmo tempo quanto há processadores. A saída é agrupada por arquivo e impressa na ordem de descoberta, qualquer que tenha sido a ordem de término dos processos, e as cores seguem o terminal e `NO_COLOR`.

<!-- @include: ../../../.fragments/tooling/cli/test-options.md -->

## Relatório

Cada arquivo imprime uma linha por teste, indentada sob as suas suítes, seguida de um resumo. Um teste que falha imprime a sua mensagem abaixo do nome, e o processo encerra com status 1 quando qualquer teste falha.

<!-- @include: ../../../.fragments/tooling/cli/test-report.md -->
