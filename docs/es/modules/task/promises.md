---
title: Tasks y Promises
description: Cuándo usar Promise.* y cuándo usar tyrus:task.*.
---

# Tasks y Promises

`Promise<T>` y `tyrus:task` se solapan lo suficiente como para que cuatro de sus nombres parezcan duplicados, `race` existe en ambos, y `joinAll` se lee como `Promise.all`. No son duplicados, y elegir el equivocado normalmente cuesta o la cancelación, o un error de compilación.

La versión corta: una `Promise<T>` es un **valor** que se va a resolver, y `Promise.*` combina valores que ya tienes en mano. Un `JoinHandle<T>` es una **tarea en ejecución**, y `tyrus:task.*` compone tareas, lo que significa que también puede detenerlas.

## Los Dos Tipos

`Promise<T>` llega de cualquier llamada `async`. Tiene `then`/`catch`/`finally`, y nada en ella dice quién está ejecutando el trabajo.

`JoinHandle<T>` llega solo de `spawn`. Nombra una tarea corriendo concurrentemente con quien la lanzó. Eso es lo que le da sentido a `abort()`; no hay nada que abortar en una `Promise<T>` que ninguna tarea posee.

Ambos se consumen exactamente una vez. `await pending` consume una `Promise<T>`, y esperar el mismo binding de nuevo es `E3006`; `await handle.join()` consume un `JoinHandle<T>`, y unirlo de nuevo, o abortarlo después, es `E3033`. En ambos casos el valor ya no existe tras el primer consumo, así que guarda el resultado en un binding y lee ese binding.

## Eligiendo un Combinador

| Objetivo | Usa |
|------|-----------|
| Esperar varias promises que ya tienes | [`Promise.all`](../../language/async-await.md) |
| Esperar varias promises y conservar cada fallo | `Promise.allSettled` |
| Tomar la primera promise en resolverse, con éxito o fallo | `Promise.race` |
| Tomar la primera promise en *tener éxito* | `Promise.any` |
| Esperar varias tareas lanzadas | [`joinAll`](./join-all.md) |
| Tomar la primera tarea lanzada en resolverse y cancelar las demás | [`race`](./race.md) |
| Limitar cuánto tiempo puede tardar una promise | [`timeout`](./timeout.md) |
| Detener una promise pendiente desde afuera | [`cancel`](./cancel.md) |

La diferencia entre los dos `race` es la más importante. `Promise.race` se resuelve con la promise que se resolvió primero y deja las otras corriendo hasta el final, su trabajo igual ocurre, y su resultado simplemente se ignora. `tyrus:task.race` se resuelve de la misma forma y luego **cancela todos los demás handles**, porque recibió tareas en vez de valores, y una tarea es algo que puede detenerse.

## Por Qué `joinAll` No Es `Promise.all`

`Promise.all` recibe `Promise<T>[]`; `joinAll` recibe `JoinHandle<T>[]`. Uno no sustituye al otro, y el compilador lo dice en vez de convertir en silencio. `joinAll` también consume todo handle que recibe, exactamente como lo hace `join()`, así que un handle pasado a él no puede unirse de nuevo después.

## Durmiendo y Cancelando

`sleepAsync` y `cancel` viven en `tyrus:task` y no en `Promise` porque ambos son sobre el runtime, no sobre un valor: `sleepAsync` devuelve el hilo de trabajo en vez de retenerlo, y `cancel` resuelve una promise pendiente desde afuera de la tarea que la produjo. `cancel` es la única función de `tyrus:task` que recibe una `Promise<T>`, la cancelación alcanza el valor, no solo la tarea.
