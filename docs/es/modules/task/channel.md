---
title: channel
description: Crea un canal tipado y mueve valores entre tareas concurrentes a través de los extremos Sender y Receiver.
---

# channel

`channel` crea un canal tipado y retorna sus dos extremos propios (owned) como una tupla.

<SignatureBlock label="Firma">channel&lt;T&gt;(capacity?: usize): [Sender&lt;T&gt;, Receiver&lt;T&gt;]</SignatureBlock>

`capacity` es opcional: omítelo para un canal ilimitado, o pasa un `usize` para acotar cuántos valores quedan en buffer, `Sender<T>.send`/`trySend` esperan por (o rechazan por falta de) espacio cuando un canal acotado está lleno. `Sender<T>.send(value: T): Promise<void>` espera por espacio y entrega `value`, rechazando con `channel is closed` en cuanto todo `Receiver<T>` sea descartado; `Sender<T>.trySend(value: T): boolean` nunca espera, retornando `true` cuando se acepta o `false` si el canal está lleno o cerrado. `Receiver<T>.recv(): Promise<T?>` espera el siguiente mensaje, resolviéndose a `None` en cuanto todo `Sender<T>` fue descartado y no queda nada en buffer; `Receiver<T>.tryRecv(): T?` nunca espera, retornando el siguiente mensaje en buffer o `None` si todavía no hay uno. `Receiver<T>.close(): void` cierra el canal, `send`/`trySend` empiezan a fallar, mientras `recv`/`tryRecv` todavía drenan lo que ya estaba en buffer antes de asentarse en `None`. Al igual que `Promise<T>` y `JoinHandle<T>`, `Sender<T>`/`Receiver<T>` son tipos nativos y nunca necesitan importarse.

Los payloads de canal quedan restringidos al mismo subconjunto que la propia `Promise<T>` soporta: `string`, arrays de elementos `Copy` y escalares `Copy`, no interfaces arbitrarias.

Un `Sender<T>`/`Receiver<T>` movido dentro de una tarea `spawn` se consume ahí, igual que cualquier otro valor propio (owned), capturarlo dos veces, digamos, en dos tareas separadas, es un error de move en tiempo de compilación, así que un único productor o consumidor por extremo es la forma a buscar. El ejemplo de abajo mueve `tx` dentro de un worker generado (spawned) que devuelve un resultado por host apenas se resuelve, dejando que `main` procese los resultados a medida que llegan en vez de esperar a que toda búsqueda termine, como haría `Promise.all`, mira la página de Playground en Tooling para entender por qué este ejemplo no corre ahí:

<!-- @include: ../../../.fragments/modules/task/channel.md -->

Una capacidad de `0` es un error de compilación (`E3035`): un canal sin espacio no guarda nada, así que todo `send` esperaría por un `recv` que no tiene de dónde tomar el valor. Omite el argumento para usar la capacidad predeterminada.
