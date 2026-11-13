---
title: Async y Await
description: Funciones async, expresiones await, Promise<T>, combinadores y cancelación cooperativa en tyrus.
---

# Async y Await

tyrus admite funciones `async` ejecutables, expresiones `await`, valores nativos `Promise<T>` y manejo estructurado de rechazos.

## Funciones Async

Declara una función async con la palabra clave `async`. Retorna un `Promise<T>` que se resuelve al tipo interno.

<!-- @include: ../../.fragments/language/async-await/async-function.md -->

Una función async con retorno `void` retorna `Promise<void>` y no porta un payload.

<!-- @include: ../../.fragments/language/async-await/void-async.md -->

## Await de nivel superior

Una `async function main(): Promise<void>` sin parámetros puede suspenderse en múltiples declaraciones `await` de nivel superior.

<!-- @include: ../../.fragments/language/async-await/top-level-await.md -->

## Funciones flecha async

Las funciones flecha async siguen la misma sintaxis.

<!-- @include: ../../.fragments/language/async-await/arrow-async.md -->

## Objetos mutables a través de await

Un objeto mutable cuyos campos almacenados obligatorios sean todos valores `Copy` puede permanecer vivo a través de suspensiones. Sus campos `Copy` se pueden modificar in-place, y el binding completo se puede reemplazar por otro literal de objeto compatible.

<!-- @include: ../../.fragments/language/async-await/mutable-copy-object.md -->

Los objetos mutables con campos heap-owned compatibles pueden reemplazarse por completo o actualizarse campo por campo. La asignación a campos string, array, optional y objetos anidados evalúa y clona profundamente el reemplazo antes de liberar el campo anterior, por lo que las asignaciones derivadas del propio valor siguen siendo válidas. Los métodos mutables de campos de arrays de strings, como `push`, `splice`, `fill` y `sort`, operan directamente sobre el campo poseído por el frame. Cada local recursivamente owned lleva un bit de inicialización, y el drop glue tipado generado libera solo los valores inicializados exactamente una vez durante el reemplazo o la destrucción del frame.

<!-- @include: ../../.fragments/language/async-await/mutable-owned-object.md -->

## Flujo de control anidado

Las suspensiones pueden aparecer a través de bloques, ramas condicionales, bucles, switches y manejo estructurado de errores anidados arbitrariamente. La máquina de estados asíncrona conserva las instancias de bucle activas y los ámbitos léxicos entre rutas hermanas con suspensión, por lo que `break`, `continue`, los bindings locales de rama y las instrucciones anteriores o posteriores a cada suspensión mantienen su comportamiento normal. Ambas ramas de un `if` anidado pueden contener cadenas de suspensión independientes con distintas longitudes, tipos de payload y bindings; la condición se evalúa una sola vez.

<!-- @include: ../../.fragments/language/async-await/nested-control-flow.md -->

## Manejo de errores

Usa `try`/`catch`/`finally` para manejar rechazos async.

Cuando un `finally` suspensivo, incluido uno anidado recursivamente dentro de bucles y ramas, comienza con un `return`, `break`, `continue` o `throw` pendiente, el frame asíncrono conserva esa completion hasta que termina el cleanup. Una completion producida dentro de `finally` reemplaza la pendiente, y el rechazo de una operación de cleanup esperada tiene prioridad. Cada `finally` se ejecuta exactamente una vez.

::: danger error
<!-- @include: ../../.fragments/language/async-await/error-handling.md -->
:::

Un `throw` dentro de una función async rechaza el Promise que retorna.

::: danger error
<!-- @include: ../../.fragments/language/async-await/throw-async.md -->
:::

## AggregateError

Cuando múltiples operaciones fallan, `AggregateError` agrupa todas las razones de rechazo.

::: danger error
<!-- @include: ../../.fragments/language/async-await/aggregate-error.md -->
:::

## Promise.resolve y Promise.reject

Crea promises resueltos o rechazados directamente.

<SignatureBlock label="Firma">Promise.resolve&lt;T&gt;(value: T): Promise&lt;T&gt;</SignatureBlock>

<SignatureBlock label="Firma">Promise.reject&lt;T&gt;(reason: Error): Promise&lt;T&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/async-await/promise-resolve-reject.md -->

## Promise.all

Espera a que todos los promises se resuelvan. Rechaza inmediatamente si algún promise se rechaza.

<SignatureBlock label="Firma">Promise.all&lt;T&gt;(values: Promise&lt;T&gt;[]): Promise&lt;T[]&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/async-await/promise-all.md -->

## Promise.allSettled

Espera a que todos los promises se resuelvan o rechacen. Nunca rechaza.

<SignatureBlock label="Firma">Promise.allSettled&lt;T&gt;(values: Promise&lt;T&gt;[]): Promise&lt;PromiseSettledResult&lt;T&gt;[]&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/async-await/promise-all-settled.md -->

## Promise.race

Resuelve o rechaza con el primer promise que se resuelva.

<SignatureBlock label="Firma">Promise.race&lt;T&gt;(values: Promise&lt;T&gt;[]): Promise&lt;T&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/async-await/promise-race.md -->

## Promise.any

Resuelve con el primer promise exitoso. Rechaza solo si todos los promises se rechazan.

<SignatureBlock label="Firma">Promise.any&lt;T&gt;(values: Promise&lt;T&gt;[]): Promise&lt;T&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/async-await/promise-any.md -->

## Promise.withResolvers

Crea un promise con funciones externas de resolve y reject.

<SignatureBlock label="Firma">Promise.withResolvers&lt;T&gt;(): { promise: Promise&lt;T&gt;, resolve: (value: T) =&gt; void, reject: (reason: Error) =&gt; void }</SignatureBlock>

<!-- @include: ../../.fragments/language/async-await/promise-with-resolvers.md -->

## then, catch y finally

Encadena continuaciones de promise sin `await`.

<!-- @include: ../../.fragments/language/async-await/then-catch-finally.md -->

## Cancelación cooperativa

Usa `cancel` de `tyrus:task` para cancelar cooperativamente un promise pendiente.

<!-- @include: ../../.fragments/language/async-await/cancellation.md -->

## Sleep

Retrasa la ejecución sin bloquear el hilo executor.

<!-- @include: ../../.fragments/language/async-await/sleep.md -->

## Combinadores de Promise en la práctica

<!-- @include: ../../.fragments/language/async-await/promise-combinators-practice.md -->

## Esperando Una Vez

Una `Promise<T>` es consumida por el `await`. El runtime mantiene una única referencia a ella, y el `await` toma esa referencia y la libera, así que esperar el mismo binding dos veces es un error de compilación (`E3006`) y no una segunda lectura de un valor ya resuelto:

<!-- @include: ../../.fragments/language/async-await/await-once.md -->

Guarda el valor esperado en un binding y lee el binding. La misma regla aplica a `JoinHandle<T>` y `.join()`.

## Ejemplos de Error

::: danger La función async debe retornar Promise
Una función async que retorna un tipo que no es Promise es un error de compilación:

<!-- @include: ../../.fragments/language/async-await/err-bad-return.md -->

<!-- @include: ../../.fragments/language/async-await/err-bad-return-output.md -->

:::

::: warning Promise descartado
Un Promise que no se espera, retorna ni almacena produce una advertencia:

<!-- @include: ../../.fragments/language/async-await/err-discarded-promise.md -->

<!-- @include: ../../.fragments/language/async-await/err-discarded-promise-output.md -->

:::

::: danger await fuera de async
Usar `await` fuera de una función async es un error de compilación:

<!-- @include: ../../.fragments/language/async-await/err-outside-async.md -->

<!-- @include: ../../.fragments/language/async-await/err-outside-async-output.md -->

:::
