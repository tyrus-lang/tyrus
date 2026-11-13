---
title: Pattern Matching
description: Expresiones match, arms con guard, wildcard, narrowing de unions y exhaustividad.
---

# Pattern Matching

`match` es una forma nativa de flujo de control de tyrus. El checker garantiza que todos los patrones estén cubiertos o exista un caso predeterminado.

Los arms usan directamente `pattern => body`. Para una única expresión devuelta, usa la forma compacta `pattern => expression;`; el punto y coma es obligatorio. Los arms con bloque siguen siendo compatibles y su punto y coma final es opcional. Los paréntesis alrededor de un patrón son opcionales para todos los patrones soportados, por lo que `"ready" =>` y `("ready") =>` son equivalentes.

<!-- @include: ../../.fragments/language/pattern-matching/pattern-matching.md -->

::: warning Un arm compacto es un `return`
`pattern => expression;` devuelve `expression` desde la función que lo contiene, exactamente como `return expression;`. Por eso, dentro de un bucle, sale de la función entera, no solo del bucle, y nada después del `match` se ejecuta. El programa de abajo imprime solo `one`. Para ejecutar el arm y continuar, usa un arm con bloque, `1 => { print("one"); }`.

<!-- @include: ../../.fragments/language/pattern-matching/compact-arm-returns.md -->
:::

## Match en Unions

<!-- @include: ../../.fragments/language/pattern-matching/match-over-unions.md -->

Los patrones soportados incluyen matches de literal, wildcard `_`, bindings de captura, arms con guard, destructuring de objeto con match de campo literal y patrones de array.

## Patrones de Array

Los patrones de array desestructuran valores `Array<T>` y `readonly Array<T>`. `[]` coincide solo con un array vacío, `[first, second]` coincide exactamente con dos elementos y `[head, ...tail]` coincide con uno o más elementos. Los bindings de elementos tienen tipo `T`; `tail` es un nuevo `Array<T>` con propiedad de los elementos restantes, equivalente a `values.slice(1)`, y no una vista prestada. Un patrón de array permite como máximo un patrón de resto, debe ser final y debe ser un binding o `_`. Los elementos literales deben ser asignables a `T`. Los patrones de array anidados y la exhaustividad por longitud aún no están disponibles porque los arrays ordinarios no llevan la longitud en el tipo.

<!-- @include: ../../.fragments/language/pattern-matching/array-patterns.md -->

## Match de Literal

Compara directamente contra valores constantes como strings, números o booleanos.

<!-- @include: ../../.fragments/language/pattern-matching/literal-match.md -->

## Patrones de Range

Compara un valor numérico o `char` contra un intervalo acotado: `a..b` excluye `b`, `a..=b` lo incluye, y cualquiera de los límites puede omitirse para un range abierto (`..b`, `a..`). Un límite del range debe ser asignable al propio tipo del sujeto, comparar un `string` contra un range numérico es un error de compilación.

<!-- @include: ../../.fragments/language/pattern-matching/range-pattern.md -->

Un range vacío o invertido (`10..5`) es un warning, no un error, simplemente nunca coincide, igual que cualquier otro arm inalcanzable. Detectar arms superpuestos y verificar la exhaustividad para un tipo numérico finito (`u8`, por ejemplo) son decisiones deliberadas de alcance, no vacíos: ambos siguen abiertos, como refinamientos rastreados, no comportamiento obligatorio.

## Patrones de Binding

Captura valores coincidentes en variables para usar en el cuerpo del arm.

<!-- @include: ../../.fragments/language/pattern-matching/binding-pattern.md -->

## Patrón Wildcard

Usa `_` para coincidir con cualquier valor que no fue coincidido por arms anteriores.

<!-- @include: ../../.fragments/language/pattern-matching/wildcard-pattern.md -->

## Arms con Guard

Agrega condiciones `if` en arms para coincidencia más específica.

<!-- @include: ../../.fragments/language/pattern-matching/guard-pattern.md -->

## Destructuring de Objeto

Desestructura objetos y coincide con valores específicos de campos.

<!-- @include: ../../.fragments/language/pattern-matching/object-destructure.md -->

## Patrones Combinados

Combina narrowing de tipo, guards y match de literal para lógica compleja.

<!-- @include: ../../.fragments/language/pattern-matching/combined-patterns.md -->

## Patrones de Variante de Enum

Haz match de un enum algebraico por variante y liga un nombre por cada campo del payload. Una variante con dos campos exige dos bindings: un único nombre para un payload de varios campos se rechaza, porque no hay forma de leer los campos de vuelta a partir de él. Una variante sin payload no acepta ningún binding.

<!-- @include: ../../.fragments/language/pattern-matching/enum-variant-pattern.md -->

## Exhaustividad

Para subjects cerrados como enum, boolean, literal union y tagged union, `match` debe cubrir toda variante o incluir `_`. Arms inalcanzables se reportan como warnings.

::: tip Switch está documentado en Flujo de control
Usa [`switch`](./control-flow.md#switch-y-match) para dispatch literal sobre valores primitivos. Usa `match` cuando importan destructuring, narrowing o exhaustividad.
:::
