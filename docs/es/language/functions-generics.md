---
title: Funciones, Closures y Generics
description: Arrow functions, valores de función, closures, funciones genéricas, constraints y monomorfización.
---

# Funciones, Closures y Generics

Tyrus ofrece arrow functions y valores de función nativos.

## Funciones

<!-- @include: ../../.fragments/language/functions-generics/functions.md -->

Las firmas de función incluyen tipos de parámetros y retorno. Los cuerpos de expresión retornan implícitamente. Una función non-`void` debe retornar un valor en todo camino alcanzable.

## Valores de Función

<!-- @include: ../../.fragments/language/functions-generics/function-values.md -->

Los valores de función pueden almacenarse y llamarse. Pueden capturar valores léxicos, sujetos a reglas de ownership y lifetime.

Una función nombrada es en sí misma un valor de primera clase: puede pasarse como argumento, asignarse a otro binding y usarse como callback de array. Un `const nombre = (…) => …` en el nivel superior declara una función nombrada, así que lo mismo aplica a ella.

<!-- @include: ../../.fragments/language/functions-generics/function-values-passing.md -->

Una función también puede devolver una función. El valor devuelto conserva su tipo de función, por lo que el binding que lo recibe sigue siendo invocable sin anotación.

<!-- @include: ../../.fragments/language/functions-generics/make-adder.md -->

La función devuelta también puede llamarse directamente, sin un binding intermedio. El callee pasa a ser una expresión, y no un nombre, así que no lleva una firma que consultar: su tipo es el contrato completo, y llamar algo que no es un valor de función da `E0618` con el rango en el callee.

<!-- @include: ../../.fragments/language/functions-generics/immediate-call.md -->

::: warning La captura de dos niveles aún no está soportada
Una closure que captura un binding de dos niveles arriba, como en `(a) => (b) => (c) => a + b + c`, es rechazada con `E0435`, anclado en la mención que llega demasiado lejos. Una closure solo puede capturar de la función inmediatamente alrededor de ella.
:::


## Parámetros Mutables

<!-- @include: ../../.fragments/language/functions-generics/mutable-parameters.md -->

`mut` permite reasignación dentro del cuerpo de la función sin volver mutable el binding del llamador.

## Parámetros Rest

<!-- @include: ../../.fragments/language/functions-generics/rest-parameters.md -->

Un parámetro rest (`...nombre: T[]`) debe ser el último parámetro y recolecta los argumentos finales en un array. Acepta argumentos individuales, un `...spread` de un array existente, ningún argumento, y cualquier combinación junto a parámetros fijos iniciales. Los argumentos individuales se comprueban contra el tipo del elemento; un argumento spread se comprueba contra el tipo del array completo.

## Closures

<!-- @include: ../../.fragments/language/functions-generics/closures.md -->

Los valores de función usan una representación nativa de closure con puntero de función y puntero de entorno. Callbacks síncronos de arrays funcionales pueden capturar valores léxicos sin transferir ownership del recurso capturado.

### Qué Ve una Captura

Una closure que nunca sale de la función que la declaró captura los bindings mismos. Las lecturas ven el valor actual, las escrituras alcanzan el binding de afuera, y el estado se acumula entre llamadas:

<!-- @include: ../../.fragments/language/functions-generics/confined-capture.md -->

Una closure solo permanece dentro de su función cuando toda mención a ella es una llamada. Devolverla, pasarla como argumento, aliasarla con otro nombre o guardarla en cualquier lugar entrega su valor a algo que puede sobrevivir a la función, y entonces los bindings que capturó ya no existen cuando ella corre.

Una closure así captura una *copia*, tomada en el momento en que se construye. Leer una copia está bien definido. Escribirla no sirve de nada: la escritura cae en la copia, y nada de afuera puede observarla. Eso es `E0436`, anclado en la escritura misma:

<!-- @include: ../../.fragments/language/functions-generics/escaping-capture.md -->

::: tip
La corrección es devolver el valor en lugar de escribirlo, o guardar el estado compartido en un [`Mutex`](../modules/task/mutex.md) que ambos lados alcancen. Lo segundo es lo que necesita una task iniciada con [`spawn`](../modules/task/spawn.md), y la misma regla es lo que impide que una task comparta estado mutable con el código que la inició.
:::

## Generics

<!-- @include: ../../.fragments/language/functions-generics/generics.md -->

Las funciones genéricas usan monomorfización nativa: el compilador emite una especialización LLVM por combinación concreta de tipos. La sustitución genérica funciona en arrays, referencias, opcionales y tipos de función.
