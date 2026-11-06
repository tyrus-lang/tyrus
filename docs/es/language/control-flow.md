---
title: Flujo de control
description: Condicionales, loops, switch, match, return, throw y try/catch/finally en tyrus.
---

# Flujo de control

Tyrus soporta formas familiares de statements para branches, loops y salidas anticipadas. La sintaxis es concisa, mientras el checker valida inicialización, ownership y caminos de retorno.

## Condicionales

<!-- @include: ../../.fragments/language/control-flow/conditionals.md -->

La condición es un `boolean`, o un `T?` usado como test de presencia: un opcional es verdadero cuando tiene valor. Lo mismo aplica a `while`, `for`, el ternario `? :`, `&&`, `||` y `!`. En la rama donde el opcional se sabe presente, el cuerpo de un `if`, el lado verdadero de un ternario o el lado derecho de un `&&`, se estrecha a su tipo no-opcional.

## Condiciones y statements de cortocircuito

<!-- @include: ../../.fragments/language/control-flow/conditions-and-short-circuit.md -->

`cond && effect();` y `cond || effect();` también son statements válidos: ejecutan el lado derecho por su efecto de cortocircuito, así que el lado derecho puede retornar `void`.

## while

<!-- @include: ../../.fragments/language/control-flow/while-loop.md -->

## do...while

<!-- @include: ../../.fragments/language/control-flow/do-while-loop.md -->

`do...while` ejecuta el cuerpo al menos una vez.

## loop

<!-- @include: ../../.fragments/language/control-flow/loop.md -->

`loop` repite su cuerpo para siempre hasta un `break`. A diferencia de `while`, nunca evalúa una condición, así que es la forma natural para bucles que salen a mitad del cuerpo. Cuando todo `break` del cuerpo lleva un valor, el `loop` es una expresión: se evalúa al valor pasado al `break` que lo termina, por lo que `loop { break 42; }` tiene tipo `i32`. `continue` salta a la siguiente repetición. Un `break` con valor solo se permite dentro de una expresión `loop` (`E0571`); `break` sin valor funciona como en cualquier otro bucle.

## for clásico

<!-- @include: ../../.fragments/language/control-flow/classic-for-loop.md -->

Un contador escrito sin anotación toma el tipo del límite del bucle: la `i` de arriba es el `usize` que produce `length`, así que indexa el array directamente y no necesita anotación propia. Un contador medido contra un límite con signo sigue siendo `i32`, lo que mantiene válida la aritmética que baja de cero. Anota el contador cuando quieras elegir el tipo tú mismo; la anotación siempre gana.

## for...of

<!-- @include: ../../.fragments/language/control-flow/for-of.md -->

Usa `for...of` para arrays y valores iterables de runtime. El binding puede desestructurar cada elemento con un patrón de array/tupla `[left, right]` o de objeto `{ x, y }`, lo que combina bien con las entries `[clave, valor]` de un `Map`.

<!-- @include: ../../.fragments/language/control-flow/for-of-destructuring.md -->

`for...of` también itera un `Range` directamente, asociando cada uno de sus valores numéricos por turno, `a..b` excluye `b`, `a..=b` incluye.

<!-- @include: ../../.fragments/language/control-flow/for-of-range.md -->


## for...in

<!-- @include: ../../.fragments/language/control-flow/for-in.md -->

`for...in` itera sobre los nombres de los campos de un objeto. Un array no es un tipo objeto, así que usa `keys()` con `for...of` cuando el índice es el valor que necesitas; los índices de array son `usize` en la API nativa de colecciones.

<!-- @include: ../../.fragments/language/control-flow/for-in-array-keys.md -->

## break y continue

<!-- @include: ../../.fragments/language/control-flow/break-and-continue.md -->

`break` sale del loop o `switch` envolvente más cercano. `continue` avanza el loop envolvente más cercano y también funciona dentro de un `switch` o `match` anidado. Un arm de `match` nunca tiene fallthrough y rechaza un `break` que podría alcanzar una estructura de control externa (`E0269`). Un `break` fuera de todo loop y `switch`, o un `continue` fuera de todo loop, es un error (`E0268`).

## return

<!-- @include: ../../.fragments/language/control-flow/return.md -->

El checker rechaza funciones con retorno non-`void` que pueden terminar sin devolver un valor.

## `defer` y `errdefer`

<!-- @include: ../../.fragments/language/control-flow/defer.md -->

`defer` registra una expresión o bloque para ejecutarse cuando termina su ámbito léxico actual. `errdefer` tiene las mismas dos formas, pero se ejecuta solo cuando el ámbito termina con un retorno explícito de error o al propagar un error con `?`. Ambos son palabras clave globales del lenguaje y no necesitan import. Los finalizadores se ejecutan en orden LIFO; en una salida de error, los `errdefer` del ámbito se ejecutan antes que los `defer`. Los ámbitos anidados terminan antes que su ámbito padre.

El cuerpo captura valores en su punto de registro. No puede usar `return`, `break`, `continue`, `throw`, `await` ni `?`, porque la limpieza no puede sustituir la salida en curso. `errdefer` requiere un retorno `Result<T, E>` o `T?`. Los finalizadores se admiten actualmente solo en funciones síncronas. `panic()` aborta inmediatamente y no ejecuta ninguno de los finalizadores, por lo que la limpieza recuperable debe usar una ruta estructurada de error.

## switch y match

`switch` es una instrucción de flujo de control que compara un valor con cases literales. Úsalo para dispatch simple sobre valores primitivos o enums.

<!-- @include: ../../.fragments/language/switch/basic-switch.md -->

::: tip Usa switch para dispatch simple
Usa `switch` cuando el subject es un valor primitivo y cada branch es una comprobación de igualdad literal.
:::

### Fallthrough y break

La ejecución continúa desde el case coincidente hacia cada body siguiente. Usa `break` para salir del `switch` (o loop) más cercano. Omitirlo es útil cuando varios cases comparten comportamiento:

<!-- @include: ../../.fragments/language/switch/fallthrough.md -->

Con `"admin"`, esto imprime `privileged` y `authenticated`. Un `continue` dentro de un `switch` todavía avanza el loop envolvente más cercano. A diferencia de `switch`, [`match`](./pattern-matching.md) nunca tiene fallthrough.

### Cases duplicados

El linter reporta cases repetidos porque el segundo branch es inalcanzable.

::: danger error
<!-- @include: ../../.fragments/language/switch/duplicate-case.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/switch/duplicate-case-error.md -->
:::

### Switch vs Match

Usa [`match`](./pattern-matching.md) cuando el branch necesita destructuring, guarded arms o narrowing sobre una tagged union. Usa `switch` para hacer dispatch sobre un enum o un valor literal.

<!-- @include: ../../.fragments/language/switch/enum-switch.md -->

::: warning Los patrones de match no son miembros de enum
Un patrón de `match` es un wildcard, un binding, un object pattern o un literal. Un miembro de enum como `PaymentStatus.Paid` no es un patrón, así que haz el dispatch sobre un enum con `switch` y reserva `match` para tagged unions y guarded arms.
:::

## throw y try/catch/finally

<!-- @include: ../../.fragments/language/control-flow/try-catch-finally.md -->

Un `catch` sin tipo vincula `Error`, la interfaz con `message` y `code` que usan los errores recuperables, así que `error.message` y `error.code` están disponibles sin anotación. Un `string` lanzado, o cualquier otro tipo, sigue necesitando el tipo escrito, como en `catch (error: string)`.

<!-- @include: ../../.fragments/language/control-flow/catch-error.md -->

El bloque `finally` se ejecuta antes de que el control salga de `try` o `catch`, incluso por `return`, `throw`, `break` o `continue`.

::: warning Evita flujo inseguro en finally
Retornar, lanzar, dar break o continue en `finally` puede sobrescribir el flujo original y es reportado por reglas de lint.
:::
