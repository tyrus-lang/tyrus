---
title: joinAll
description: Espera a que cada JoinHandle de un array se asiente, manteniendo los resultados en orden.
---

# joinAll

`joinAll` espera a que cada `JoinHandle<T>` de un array se asiente, resolviéndose a sus resultados en el mismo orden en que se pasaron los handles.

<SignatureBlock label="Firma">joinAll&lt;T&gt;(handles: JoinHandle&lt;T&gt;[]): Promise&lt;T[]&gt;</SignatureBlock>

Se resuelve en cuanto cada handle ya se resolvió, sin importar cuál terminó primero de verdad, `results[0]` siempre es el resultado de `handles[0]`, no el que se asentó más temprano. Se rechaza con el primer rechazo que observa, de la misma forma que `Promise.all` lo hace. Todo handle pasado a `joinAll` se consume igual que `.join()` consume uno, reutilizar un handle ya pasado a `joinAll`, a `race`, o a su propio `.join()`, se rechaza en tiempo de compilación (`E3033`).

<!-- @include: ../../../.fragments/modules/task/join-all.md -->
