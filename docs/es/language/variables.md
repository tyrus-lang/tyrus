---
title: Variables
description: Declaración de variables con let y const, mutabilidad, inmutabilidad profunda y bindings opcionales.
---

# Variables

Tyrus proporciona dos palabras clave para la declaración de bindings: `const` y `let`. Ambas imponen un ámbito léxico estricto y participan en la comprobación estática de tipos, pero difieren en mutabilidad, reglas de reasignación y requisitos de inicialización.

## const vs let

La elección entre `const` y `let` establece si un binding y sus datos subyacentes pueden modificarse.

| Característica | `const` | `let` |
|---|---|---|
| Reasignación | Prohibida en tiempo de compilación | Permitida |
| Inmutabilidad profunda | Aplicada recursivamente en objetos y arrays | No aplicada (las propiedades y elementos son mutables) |
| Inicializador obligatorio | Siempre obligatorio | Opcional (`let x: i32;` está permitido) |
| Shadowing | Prohibido | Prohibido |

## Bindings inmutables con const

`const` declara un binding inmutable cuyo valor no se puede reasignar después de su inicialización. Utiliza `const` por defecto siempre que una variable no requiera reasignación.

<!-- @include: ../../.fragments/language/variables/const-valid.md -->

Intentar asignar un nuevo valor a un binding `const` produce un error en tiempo de compilación.

::: danger error
<!-- @include: ../../.fragments/language/variables/const-error.md -->
:::

## Inmutabilidad profunda con const

En Tyrus, `const` garantiza una inmutabilidad profunda. En Tyrus prohíbe modificar las propiedades de un objeto o los elementos de un array declarados con `const`.

<!-- @include: ../../.fragments/language/variables/deep-immutability-valid.md -->

Modificar un campo de objeto o elemento de array a través de una referencia `const` es rechazado por el compilador.

::: danger error
<!-- @include: ../../.fragments/language/variables/deep-immutability-error.md -->
:::

## Bindings mutables con let

`let` declara un binding mutable. Permite tanto la reasignación de la variable como la mutación in situ de estructuras compuestas como arrays y objetos.

<!-- @include: ../../.fragments/language/variables/let-valid.md -->

La reasignación debe seguir respetando el tipo declarado estáticamente o inferido de la variable.

::: danger error
<!-- @include: ../../.fragments/language/variables/let-error.md -->
:::

## Bindings let sin inicializador

Una variable declarada con `let` puede declararse sin un inicializador inmediato, incluso para tipos no opcionales como `let number: i32;`. Los bindings opcionales (`T?`) comienzan vacíos y se pueden evaluar de forma segura con coalescencia nula (`??`), mientras que los bindings no opcionales deben asignarse antes de poder leerse.

<!-- @include: ../../.fragments/language/variables/uninitialized-valid.md -->

Leer una variable no opcional sin inicializar antes de la asignación es un error, y declarar un `const` sin inicializador está prohibido.

::: danger error
<!-- @include: ../../.fragments/language/variables/uninitialized-error.md -->
:::

## Shadowing no permitido

Tyrus no permite el shadowing de variables en el mismo ámbito. Para actualizar un valor, declare una vinculación mutable con `let` y realice una reasignación en lugar de volver a declarar la variable.

<!-- @include: ../../.fragments/language/variables/shadowing-valid.md -->

Intentar volver a declarar una variable con `let` o `const` en el mismo ámbito es rechazado por el compilador.

::: danger error
<!-- @include: ../../.fragments/language/variables/shadowing-error.md -->
:::

## Inferencia de tipos y anotaciones

Los tipos de las variables pueden anotarse explícitamente o inferirse automáticamente a partir de la expresión del inicializador.

<!-- @include: ../../.fragments/language/variables/type-annotation-valid.md -->

Cuando se proporciona una anotación de tipo explícita, el valor asignado debe satisfacer la restricción de dicho tipo.

::: danger error
<!-- @include: ../../.fragments/language/variables/type-annotation-error.md -->
:::
