---
title: Runtime
description: Threads de trabalho, precedência de configuração e o que acontece com tarefas pendentes quando main termina.
---

# Runtime

Toda tarefa que um programa dispara roda sobre um runtime assíncrono compartilhado. Nada precisa ser iniciado ou configurado para que ele funcione: o runtime sobe na primeira vez que o programa aguarda algo, se dimensiona para a máquina e para quando o programa termina. Esta página cobre as duas coisas que um projeto pode decidir sobre ele, quanto da máquina ele pode usar e o que o fim dele significa para o trabalho ainda em voo.

## Threads de Trabalho

Por padrão o runtime usa uma thread de trabalho por CPU lógica. Um projeto que queira outro número define isso no `tyrus.json`:

<!-- @include: ../../../.fragments/modules/task/runtime/worker-threads.md -->

O valor é embutido no programa compilado, então ele viaja com o binário e não precisa do `tyrus.json` ao lado em tempo de execução. Uma contagem abaixo de `1` é erro de build: o programa não teria como progredir, e rejeitar isso no build é melhor do que entregar um binário que não roda.

A variável de ambiente `TYRUS_WORKER_THREADS` sobrepõe esse valor na inicialização:

<!-- @include: ../../../.fragments/modules/task/runtime/worker-threads-env.md -->

A precedência é `TYRUS_WORKER_THREADS`, depois `runtime.workerThreads`, depois uma worker por CPU lógica. A variável de ambiente vence para que um deploy consiga ajustar o programa à máquina em que ele realmente caiu sem recompilá-lo.

::: tip
Um valor bem acima do número de CPUs lógicas que o runtime detecta é respeitado, mas reportado em stderr. Sobrecarregar uma máquina é ocasionalmente deliberado, então isso é um aviso sobre um provável engano, e não um limite que o runtime impõe. Um valor que não seja um inteiro positivo é ignorado do mesmo jeito, com um aviso, nunca de forma fatal.
:::

## Shutdown

O tempo de vida assíncrono de um programa é o do `main`. Quando o `main` se resolve, o runtime para e toda tarefa ainda pendente é cancelada em vez de aguardada, não existe tarefa destacada que sobreviva ao programa.

<!-- @include: ../../../.fragments/modules/task/runtime/abandoned-task.md -->

Esse programa imprime `done` e sai. A tarefa disparada é cancelada onde estava. Rode com `TYRUS_REPORT_PENDING_TASKS=1` e o runtime diz isso em stderr:

<!-- @include: ../../../.fragments/modules/task/runtime/abandoned-task-warning.md -->

A forma de manter uma tarefa viva é esperar por ela antes de retornar:

<!-- @include: ../../../.fragments/modules/task/runtime/joined-task.md -->

::: tip
O relato é opt-in em vez de um aviso padrão, porque um programa correto pode terminar com tarefas pendentes de propósito: [`race`](./race.md) e [`timeout`](./timeout.md) cancelam as tarefas que não escolheram. Ligue-o quando uma tarefa parecer não ter rodado nada, isso é quase sempre um `join` esquecido. Como todo relato de runtime, ele vai para stderr, então o `stdout` continua sendo exatamente o que o programa imprimiu.
:::

## Loops Que Nunca Cedem

Uma thread de trabalho roda uma tarefa por vez. Um loop que não consegue nem terminar nem aguardar mantém sua thread enquanto o programa viver, e toda outra tarefa atribuída àquela thread espera atrás dele. O compilador avisa sobre o formato óbvio (`E3032`): um loop numa função `async` cuja condição é o literal `true` e cujo corpo não tem `await`, `break`, `return` nem `throw`.

<!-- @include: ../../../.fragments/modules/task/runtime/busy-wait.md -->

`await sleepAsync(0)` já basta para devolver a thread uma vez por iteração. Um loop com uma condição real é deixado em paz, mesmo quando acaba rodando para sempre: um lint que dispara em loops de contagem comuns é um lint que as pessoas desligam.
