---
title: Runtime
description: Hilos de trabajo, precedencia de configuración y qué pasa con las tareas pendientes cuando main termina.
---

# Runtime

Toda tarea que un programa lanza corre sobre un runtime asíncrono compartido. Nada tiene que iniciarse ni configurarse para que funcione: el runtime arranca la primera vez que el programa espera algo, se dimensiona a la máquina y se detiene cuando el programa termina. Esta página cubre las dos cosas que un proyecto puede decidir sobre él, cuánto de la máquina puede usar y qué significa su final para el trabajo todavía en vuelo.

## Hilos de Trabajo

Por defecto el runtime usa un hilo de trabajo por CPU lógica. Un proyecto que quiera otro número lo define en `tyrus.json`:

<!-- @include: ../../../.fragments/modules/task/runtime/worker-threads.md -->

El valor se incrusta en el programa compilado, así que viaja con el binario y no necesita `tyrus.json` al lado en tiempo de ejecución. Una cuenta menor a `1` es un error de build: el programa no tendría cómo progresar, y rechazarlo en el build es mejor que entregar un binario que no puede correr.

La variable de entorno `TYRUS_WORKER_THREADS` sobrescribe ese valor al iniciar:

<!-- @include: ../../../.fragments/modules/task/runtime/worker-threads-env.md -->

La precedencia es `TYRUS_WORKER_THREADS`, luego `runtime.workerThreads`, luego un worker por CPU lógica. La variable de entorno gana para que un deploy pueda ajustar el programa a la máquina en la que realmente aterrizó sin recompilarlo.

::: tip
Un valor muy por encima del número de CPUs lógicas que el runtime detecta se respeta, pero se reporta en stderr. Sobrecargar una máquina es ocasionalmente deliberado, así que esto es una advertencia sobre un probable error, y no un límite que el runtime impone. Un valor que no sea un entero positivo se ignora igual, con una advertencia, nunca de forma fatal.
:::

## Shutdown

El tiempo de vida asíncrono de un programa es el de `main`. Cuando `main` se resuelve, el runtime se detiene y toda tarea aún pendiente se cancela en lugar de esperarse, no existe tarea desprendida que sobreviva al programa.

<!-- @include: ../../../.fragments/modules/task/runtime/abandoned-task.md -->

Ese programa imprime `done` y sale. La tarea lanzada se cancela donde estaba. Córrelo con `TYRUS_REPORT_PENDING_TASKS=1` y el runtime lo dice en stderr:

<!-- @include: ../../../.fragments/modules/task/runtime/abandoned-task-warning.md -->

La forma de mantener una tarea viva es esperarla antes de retornar:

<!-- @include: ../../../.fragments/modules/task/runtime/joined-task.md -->

::: tip
El reporte es opt-in en vez de una advertencia predeterminada, porque un programa correcto puede terminar con tareas pendientes a propósito: [`race`](./race.md) y [`timeout`](./timeout.md) cancelan las tareas que no eligieron. Actívalo cuando una tarea parezca no haber corrido nada, eso es casi siempre un `join` olvidado. Como todo reporte de runtime, va a stderr, así que `stdout` sigue siendo exactamente lo que el programa imprimió.
:::

## Loops Que Nunca Ceden

Un hilo de trabajo corre una tarea a la vez. Un loop que no puede ni terminar ni esperar mantiene su hilo mientras el programa viva, y toda otra tarea asignada a ese hilo espera detrás de él. El compilador advierte sobre la forma obvia (`E3032`): un loop en una función `async` cuya condición es el literal `true` y cuyo cuerpo no tiene `await`, `break`, `return` ni `throw`.

<!-- @include: ../../../.fragments/modules/task/runtime/busy-wait.md -->

`await sleepAsync(0)` ya basta para devolver el hilo una vez por iteración. Un loop con una condición real se deja en paz, aun cuando termine corriendo para siempre: un lint que dispara en loops de conteo comunes es un lint que la gente apaga.
