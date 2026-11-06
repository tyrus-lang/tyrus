---
title: Readonly
description: Las seis formas de readonly, propiedades de interfaz, posición de tipo, parámetros, Readonly<T>, as const y readonly const.
---

# Readonly

`readonly` trata de *mutabilidad*: dice que no se puede escribir a través de un valor, a ninguna profundidad. No trata del *rebinding*, que es lo que deciden `const` y `let`, ni de *estrechar un tipo a un literal*, que es lo que agrega `as const`.

La palabra clave aparece en seis lugares distintos. Todos producen la misma garantía, nada detrás de este valor puede recibir una asignación, y difieren solo en aquello a lo que se aplican.

| Forma | Dónde va | Qué protege |
|---|---|---|
| `readonly id: i32` | propiedad de interfaz | esa propiedad, y todo lo que hay debajo |
| `readonly Config` | posición de tipo | todo valor de ese tipo |
| `config: readonly Config` | parámetro | el argumento, mientras dure la llamada |
| `Readonly<Config>` | tipo utilitario | todas las propiedades de una interfaz a la vez |
| `[1, 2] as const` | sufijo de expresión | el valor, y estrecha cada elemento a su tipo literal |
| `readonly const PRIMES: i32[]` | declaración | el binding, en profundidad |

La garantía es recursiva en todas ellas. Un valor `readonly` un nivel más abajo sigue siendo `readonly`, así que una escritura anidada se rechaza igual que la escritura de superficie.

## `readonly` en propiedad de interfaz

La forma más estrecha: una propiedad de una interfaz. Todo lo demás de la interfaz sigue siendo escribible.

::: danger error
<!-- @include: ../../.fragments/language/readonly/interface-property.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/readonly/interface-property-diagnostic.md -->
:::

## `readonly` en posición de tipo

Escrito antes de un tipo, `readonly` produce la vista de solo lectura de él. En un array esto quita los métodos que mutan.

::: danger error
<!-- @include: ../../.fragments/language/readonly/type-position.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/readonly/type-position-diagnostic.md -->
:::

Un valor readonly tampoco puede colarse dentro de un binding mutable, lo que devolvería justamente el acceso de escritura que el tipo rechazó.

::: danger error
<!-- @include: ../../.fragments/language/readonly/type-position-transfer.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/readonly/type-position-transfer-diagnostic.md -->
:::

## `readonly` en parámetro

Un parámetro recibe `readonly` en la posición del *tipo*, no antes del nombre. `readonly config: Config` se rechaza; `config: readonly Config` es la forma.

<!-- @include: ../../.fragments/language/readonly/parameter.md -->

Esta es la firma que conviene cuando una función lee un valor que no creó: quien llama ve solo por la firma que su valor vuelve intacto.

::: danger error
<!-- @include: ../../.fragments/language/readonly/parameter-mutation.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/readonly/parameter-mutation-diagnostic.md -->
:::

## `Readonly<T>`

`Readonly<T>` aplica el calificador a todas las propiedades de una interfaz a la vez, en lugar de una por una. Es recursivo. La descripción completa, con los casos anidado y de array, está en la página [Objetos e interfaces](/es/language/objects#readonly-t).

## `as const`

`as const` hace dos cosas, y solo una de ellas es sobre mutabilidad. Congela el valor y estrecha cada elemento a su propio tipo literal: en `[3000, 3001] as const` el primer elemento tiene el tipo `3000`, no `i32`.

<!-- @include: ../../.fragments/language/readonly/as-const.md -->

::: tip `as const` no sustituye a `readonly`
Use `readonly` cuando el punto sea que nada puede escribirse. Use `as const` cuando el punto sea que los *valores literales exactos* forman parte del tipo. El congelamiento que viene con `as const` es consecuencia de ese estrechamiento, no su finalidad.
:::

## `readonly const`

`readonly const` es la forma de declaración. El binding nunca se reasigna, porque es un `const`, y nada detrás de él puede escribirse, porque es `readonly`.

<!-- @include: ../../.fragments/language/readonly/readonly-const.md -->

Un `readonly const` global cuyo inicializador es una tabla literal no cuesta nada en tiempo de ejecución: se emite en memoria de solo lectura, sin asignación, sin stores de elemento y sin nada que liberar.

`readonly const x: T` y `const x: readonly T` son la misma declaración, así que el formateador fija la primera grafía y escribe ambas así. El único lugar donde no lo hace es el inicializador de un `for`, donde la gramática solo admite `const` y `let`.

La palabra clave va antes de `const`, y el orden se verifica.

::: danger error
<!-- @include: ../../.fragments/language/readonly/readonly-const-order.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/readonly/readonly-const-order-diagnostic.md -->
:::

La anotación es obligatoria, porque no hay nada más a lo que el calificador pueda sujetarse.

::: danger error
<!-- @include: ../../.fragments/language/readonly/readonly-const-annotation.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/readonly/readonly-const-annotation-diagnostic.md -->
:::

`readonly` califica solo a `const`. Un `let` puede reasignarse, así que un `let` readonly prometería algo que la línea siguiente podría quitar, y se rechaza.

## `readonly` no es `const`

Los dos se confunden fácilmente porque un binding `const` ya es profundamente inmutable en tyrus. La diferencia aparece en cuanto el valor viaja.

`const` es una propiedad del *binding*: impide que ese nombre se reasigne e impide escrituras a través de ese nombre. No dice nada sobre el tipo, así que pasar el valor a una función que recibe un parámetro mutable está permitido.

`readonly` es una propiedad del *tipo*: viaja con el valor a parámetros, tipos de retorno, campos y otros bindings. Eso es lo que lo hace utilizable en una firma, y lo que diferencia un `readonly const` de un `const` común, la protección sobrevive a la entrega del valor.
