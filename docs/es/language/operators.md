---
title: Operadores y Expresiones
description: Numéricos, lógicos, bitwise, ternario, pipeline, typeof, type assertions, spread, conversión, parsing y process exit.
---

# Operadores y Expresiones

Tyrus mantiene operadores familiares cuando la semántica puede comprobarse estáticamente y bajarse a código nativo.

## Grupos de Operadores

| Categoría | Operadores |
| --- | --- |
| Aritméticos | `+` `-` `*` `/` `%` `**` |
| Unarios | `-` `+` `++` `--` `!` `not` `~` |
| Bitwise | `&` `\|` `^` `~` `<<` `>>` `>>>` |
| Comparación | `>` `<` `>=` `<=` `===` `!==` |
| Lógicos | `&&` `and` `\|\|` `or` |
| Ternario | `? :` |
| Asignación | `=` `+=` `-=` `*=` `/=` `%=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `>>>=` `&&=` `\|\|=` `??=` |
| Null safety | `??` `?.` |
| Borrow | `&` `&mut` |
| Pertenencia | `in` `not in` |
| Chequeo de tipo | `is` `not is` |
| Range | `..` `..=` |

`and`, `or` y `not` son equivalentes escritos exactos de `&&`, `||` y `!`, el lexer mapea cada par al mismo token, así que ambas grafías pueden mezclarse libremente en la misma expresión: `active and not suspended or isAdmin` y `active && !suspended || isAdmin` compilan de forma idéntica.

## Comparaciones Encadenadas

Tres o más de `<`, `<=`, `>`, `>=`, `===` encadenados directamente (`a < b < c`) son azúcar sintáctico para `a < b && b < c`, leyéndose más cerca de la notación algebraica que el equivalente escrito con `&&`.

<!-- @include: ../../.fragments/language/operators/chained-comparison.md -->

Cada término en la cadena se evalúa exactamente una vez, incluso cuando participa en dos comparaciones, un término con efecto secundario (una llamada a función) nunca se ejecuta dos veces. La cadena también hace cortocircuito como `&&`: en cuanto una comparación falla, los términos siguientes nunca se evalúan. Dos términos (`a < b`) no se ven afectados por este azúcar y siguen siendo una comparación simple; la cadena solo aplica a partir de tres términos. `!==` nunca encadena, no existe una lectura algebraica para `a != b != c`, así que siempre permanece como una comparación simple de dos términos.

## Pipeline

Consulta la [guía del operador pipe](./pipe-operator.md) para conocer el operador `|>`, el token topic `%` y la composición de pipelines.

## Ternario

<!-- @include: ../../.fragments/language/operators/ternary.md -->

La condición ternaria debe ser `boolean` y ambos branches deben tener el mismo tipo.

## typeof

El operador `typeof` retorna una `string` con el nombre del tipo tyrus del valor en runtime: el nombre del tipo mismo, no una categoría genérica. Los tipos numéricos siguen distintos en lugar de colapsar a `"number"`, un array informa el tipo de su elemento, una interfaz informa su propio nombre y un opcional conserva su `?`. Solo dos formas responden con una categoría, por no tener nombre que dar: un tipo de objeto anónimo (`"object"`) y una función (`"function"`).

<!-- @include: ../../.fragments/language/operators/typeof-values.md -->

`typeof` también puede usarse como type query.

<!-- @include: ../../.fragments/language/operators/typeof-type-query.md -->

Las comparaciones con `typeof` hacen narrowing de valores union en el flujo de control.

<!-- @include: ../../.fragments/language/operators/typeof-narrowing.md -->

Esto también funciona con enums, `typeof` retorna la representación subyacente:

<!-- @include: ../../.fragments/language/operators/typeof-enum.md -->

## is / not is

El operador `is` comprueba si un valor en runtime coincide con un tipo específico dentro de una union, retornando `boolean`. Es el mecanismo principal para narrowing de union types.

<!-- @include: ../../.fragments/language/operators/is-narrowing.md -->

Dentro del branch `true`, el compilador estrecha `value` a `string`. Fuera, `value` permanece `string | number`.

`not is` es la forma negada:

<!-- @include: ../../.fragments/language/operators/is-not-is.md -->

`is`/`not is` también funcionan en condiciones de `while` y asignaciones booleanas. La comprobación se ejecuta en runtime comparando la tag de tipo de la union contra el tipo objetivo.

Un error capturado conserva el tipo con el que fue lanzado, así que `is` también distingue errores. En `catch (error)`, `error is ConfigError` es `true` cuando el valor lanzado era un `ConfigError` (un error de módulo como `FsError`, o una interfaz que extiende `Error`), y dentro de ese branch `error` tiene todos los campos del subtipo. Un `catch (error: A | B)` se estrecha a la variante de la misma forma. Esto vale también en una función `async`, incluso para un error que llega como rechazo de un `await`, y en un `catch (error)` que espera antes de la comprobación. Solo un `Error` que no vino de un `catch` no puede distinguir el subtipo (`E3297`). Un handler que comprueba los errores que conoce termina con `throw error;` para el resto: el valor sigue con su subtipo, así que el siguiente `catch` todavía lo distingue, y un tipo de error añadido después se reenvía en lugar de silenciarse.

<!-- @include: ../../.fragments/language/operators/is-caught-error.md -->

## Type Assertions

<!-- @include: ../../.fragments/language/operators/type-assertions.md -->

Una assertion numérica preserva el valor o aborta; nunca wrappea. Las comprobaciones se ejecutan en todos los perfiles, así que una ejecución en debug predice lo que hace el binario publicado. Las assertions non-primitive son casts de identidad y deben usarse con cuidado.

## Conversión y Parsing

<!-- @include: ../../.fragments/language/operators/conversion-parsing.md -->

Parsing retorna valores opcionales en vez de `NaN`, preservando null safety.

La anotación elige el ancho del parseo, con el ancho elegido por la anotación. `parseInt` estrecha a cualquier tipo entero y `parseFloat`/`Number` a `f32` o `f64`; sin anotación, permanecen `i64?` y `f64?`. Un texto cuyo valor no cabe en el tipo objetivo simplemente no parsea como ese tipo, por lo que el resultado es ausencia en vez de un valor truncado.

<!-- @include: ../../.fragments/language/operators/parse-width.md -->

Un `as T` simple preserva el valor o aborta; nunca da la vuelta. Lo que cabe depende de los límites de ambos tipos, y no de su ancho, así que un `u32` guarda un valor por encima de `i32::MAX` mientras que un número negativo nunca cabe en un `u32`, ni siquiera en uno del mismo ancho.

`as T?` es un cast falible y sigue la misma regla, salvo que un valor fuera del rango de `T` se convierte en ausencia en vez de abortar, y así es como un programa se recupera de eso. Un cast de float a entero trunca hacia cero, y `NaN` produce ausencia.

<!-- @include: ../../.fragments/language/operators/fallible-cast.md -->

El `as` convierte entre números; nunca reinterpreta un valor como un tipo sin relación. Un enum se lee de vuelta como el primitivo que lo representa, pero nada más cruza: una cadena no es un número, y una interfaz no es otra ni cuando sus campos coinciden. Un valor del tipo objetivo debe ser *construido*, y afirmarlo en su lugar es un error.

::: danger error
<!-- @include: ../../.fragments/language/operators/interface-assertion-error.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/operators/validation-error-output.md -->
:::

Esto importa sobre todo donde el tipo lleva validators. Un `User` tiene la garantía de satisfacer `@min(18)`, y es esa garantía la que permite al resto del programa saltarse la comprobación. Una conversión que produjera un `User` sin ejecutar sus validators convertiría la garantía en una mentira, así que se rechaza: construye el `User`, y maneja el `ValidationError` que puede lanzar.

## Spread y Rest

El operador spread expande arrays y objetos en elementos o campos individuales:

<!-- @include: ../../.fragments/language/operators/spread.md -->

Los parámetros rest recogen argumentos trailing en un array:

<!-- @include: ../../.fragments/language/operators/rest.md -->

## Pertenencia y Ranges

`in` y `not in` comprueban si un valor pertenece a un array, un `Range`, un `Map` o un `Set`, reemplazando llamadas a `.includes()`/`!.includes()` y `.has()`/`!.has()`. `not in` es la forma negada, escrita con la palabra clave `not` en vez de un operador separado.

`a..b` es un `Range` de `a` hasta `b`, excluyendo `b`; `a..=b` incluye `b`. Un range puede anotarse como `Range<T>`, por ejemplo `const ids: Range<i32> = 1..4;`. Un `Range` tiene su propio método `.contains(x)`, `x in a..b` y `(a..b).contains(x)` son la misma comprobación.

<!-- @include: ../../.fragments/language/operators/membership-and-ranges.md -->

`in`/`not in` también conservan su significado original sobre un objeto simple o valor de interfaz: `"campo" in obj` comprueba si el tipo de `obj` declara un campo llamado `"campo"`, independiente de la comprobación de pertenencia en array/Range/Map/Set de arriba.
