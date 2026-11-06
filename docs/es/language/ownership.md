---
title: Ownership y Borrowing
description: Semántica de move, valores Copy, borrows inmutables y mutables, lifetimes, valores nullable e inicialización diferida.
---

# Ownership y Borrowing

Tyrus aplica reglas de ownership en tiempo de compilación con una sintaxis concisa.

## Move y Copy

Valores non-Copy como `string` se mueven en asignación. El binding original queda inválido después del move.

::: danger error
<!-- @include: ../../.fragments/language/ownership/move-copy.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/ownership/moved-value-diagnostic.md -->
:::

Valores primitivos como `i32`, `i64`, `f32`, `f64` y `boolean` son Copy y siguen siendo utilizables después de la asignación.

<!-- @include: ../../.fragments/language/ownership/copy-primitives.md -->

Las llamadas de función pasan argumentos read-only sin consumir bindings comunes non-Copy del llamador, manteniendo API concisas. Las referencias explícitas están disponibles cuando una API necesita exponer borrowing.

## Borrowing

<!-- @include: ../../.fragments/language/ownership/borrowing.md -->


Un préstamo puede tomar una parte del valor: `&basket.label` presta el campo. La referencia lee a través de `basket`, así que es `basket` lo que queda prestado, y asignarle mientras la referencia está viva se rechaza. Prestar dos campos distintos a la vez se rechaza por el mismo motivo, lo que es más estricto de lo necesario y nunca es incorrecto.

<!-- @include: ../../.fragments/language/ownership/borrowing-a-field.md -->
Puedes tener muchos borrows inmutables o un borrow mutable, nunca ambos al mismo tiempo. Los borrows temporales terminan al final del statement. Un borrow almacenado termina en el último punto en que su binding es mencionado, no en la llave de cierre: en cuanto nada vuelve a leer la referencia, el valor apuntado queda libre para ser asignado, prestado de forma mutable o movido, aunque el binding siga en scope. Una mención dentro del cuerpo de un bucle o capturada por una closure cuenta, y mantiene el borrow vivo mientras esa construcción pueda ejecutarse. Una mención dentro de un cuerpo de `defer` o `errdefer` mantiene el borrow vivo hasta la llave de cierre, dondequiera que esté escrito el finalizador, porque ahí es donde se ejecuta. Las referencias no pueden vivir más que el valor al que apuntan.

Dentro de una función `async` un préstamo se comporta igual, con una regla más: no puede seguir vivo en un `await`. Sostener uno a través de una suspensión se rechaza, "borrow `b` of `a` crosses `await`", y eso es justamente lo que hace baratos los préstamos ahí, ya que la máquina de estados guarda los bytes prestados en su frame y no libera ninguno.

## Non-Lexical Lifetimes (NLL)

Tyrus implementa Non-Lexical Lifetimes (NLL). En lugar de mantener los préstamos activos hasta la llave de cierre del bloque léxico (`}`), el borrow checker construye un Grafo de Flujo de Control (CFG) y utiliza análisis de liveness retrógrado (backward) para liberar préstamos inmediatamente después de su último uso real.

### Préstamo Mutable Seguido de Mutación

Un préstamo mutable libera la variable original tan pronto como la referencia ya no se lee:

<!-- @include: ../../.fragments/language/ownership/nll-mutable-borrow.md -->

Sin NLL, `r` permanecería activo hasta el final del bloque léxico, rechazando `x += 1`.

### Préstamos Dentro de Ramas Condicionales

Los préstamos tomados dentro de un bloque `if` expiran cuando la rama termina o deja de utilizarlos, liberando la variable en el scope exterior:

<!-- @include: ../../.fragments/language/ownership/nll-conditional-branches.md -->

### Move Después de Préstamo

Tan pronto como una referencia prestada ya no se accede, el valor original se puede mover o consumir normalmente:

<!-- @include: ../../.fragments/language/ownership/nll-move-after-borrow.md -->

### Reasignación de Referencias

Reasignar una referencia mutable a un nuevo destino libera de inmediato el préstamo sobre el destino anterior:

<!-- @include: ../../.fragments/language/ownership/nll-reassignment.md -->

### Prevención de Escape de Referencias

NLL solo acorta tiempos de vida donde es seguro; nunca extiende el tiempo de vida más allá del valor original. Retornar o almacenar referencias a variables locales cuyo scope termina sigue estando estrictamente prohibido:

::: danger error
<!-- @include: ../../.fragments/language/ownership/nll-escape-prevention.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/ownership/nll-escape-prevention-diagnostic.md -->
:::

## Valores Nullable

tyrus no tiene `undefined`, y el código de usuario no debe usar `null` como inicializador normal. Los valores nullable usan `T?`, campos opcionales, parámetros opcionales, fronteras externas, narrowing y `??`.

<!-- @include: ../../.fragments/language/ownership/nullable-values.md -->

Forced unwrap (`value!`) se desalienta intencionalmente y es diagnosticado por el tooling del lenguaje; prefiere narrowing explícito o valores fallback.

## Inicialización Diferida

<!-- @include: ../../.fragments/language/ownership/deferred-init.md -->

El compilador ejecuta definite-assignment analysis y rechaza leer un binding `let x: T;` antes de que todos los caminos lo asignen. Los bindings `const` deben inicializarse donde se declaran.
