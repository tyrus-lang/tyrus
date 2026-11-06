---
title: spawn
description: Ejecuta una tarea async en paralelo y obtiene un JoinHandle para esperar su resultado.
---

# spawn

`spawn` ejecuta una tarea `async` en paralelo con quien la llamó y retorna un `JoinHandle<T>` para ella.

<SignatureBlock label="Firma">spawn&lt;T&gt;(task: () =&gt; Promise&lt;T&gt;): JoinHandle&lt;T&gt;</SignatureBlock>

`JoinHandle<T>.join(): Promise<T>` se resuelve al resultado de la tarea, y `JoinHandle<T>.abort(): void` cancela cooperativamente la tarea a través del mismo mecanismo de `cancel`. `.join()` consume el handle, consumiendo el handle, así que hacer join dos veces al mismo handle, o llamar `.abort()` después de `.join()`, se rechaza en tiempo de compilación (`E3033`); `.abort()` en sí no consume el handle, así que puede llamarse cualquier cantidad de veces antes de `.join()`. Un error lanzado dentro de una tarea creada por `spawn` rechaza la `Promise<T>` que `join()` retorna, exactamente igual que cualquier otra función async. Al igual que `Promise<T>`, `JoinHandle<T>` es un tipo nativo y nunca necesita importarse.

`await spawn(...)` directamente, y `await spawn(...).join()` encadenado en la misma expresión, son ambos rechazados en tiempo de compilación (`E3031`): los dos descartan la única forma de guardar la tarea y hacer otro trabajo mientras corre en segundo plano. Un `JoinHandle<T>` descartado, `spawn(...);` como statement suelto, sin guardar el retorno, se rechaza de la misma forma que una `Promise<T>` no usada, ya que el resultado o el rechazo de la tarea nunca podrían observarse.

Cada llamada a `resolve` de `tyrus:net` abajo es una ida y vuelta de red real que puede tardar. Lanzar las dos búsquedas con `spawn` deja que la segunda empiece antes de que la primera responda, en vez de esperar su turno, mira la página de Playground en Tooling para entender por qué este ejemplo no corre ahí:

<!-- @include: ../../../.fragments/modules/task/spawn.md -->
