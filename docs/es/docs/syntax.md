---
title: Vista General de la Sintaxis
description: Una vista general corta de la sintaxis de tyrus.
---

# Vista General de la Sintaxis

tyrus usa sintaxis concisa mientras conserva las garantías del lenguaje. El análisis semántico aplica comprobaciones nativas de inicialización, nullability, ownership y flujo de control.

## Variables

<!-- @include: ../../.fragments/docs/syntax-variables.md -->

Los bindings `const` son inmutables después de la inicialización. Los bindings `let` pueden reasignarse cuando el nuevo valor es compatible con el tipo declarado.

## Funciones

<!-- @include: ../../.fragments/docs/syntax-functions.md -->

Las funciones usan tipos explícitos para parámetros y retorno. Una función que no retorna `void` debe retornar un valor en todo camino alcanzable.

`Result<T, E>` es un valor de tipo que representa éxito o error. Consulta [Valores Result](../../language/types.md#valores-result) para las reglas de payload, constructores, narrowing y propagación.

## Objetos

<!-- @include: ../../.fragments/docs/syntax-objects.md -->

Las interfaces describen formas de objetos. Los literales de objeto se comprueban estructuralmente contra los campos declarados.

## Módulos

Un módulo publica nombres con `export`, ya sea en la propia declaración o mediante una cláusula separada que puede renombrarlos. Un nombre por módulo puede ser la exportación default.

<!-- @include: ../../.fragments/docs/syntax/module-exports.md -->

`export protect` restringe quién puede importar un nombre. Un nombre protegido es visible en la carpeta que lo declara y en todas sus subcarpetas, nunca en la carpeta padre ni en una carpeta hermana. La regla depende solo del archivo que declara, por lo que mover una carpeta completa preserva todas las relaciones dentro de ella.

<!-- @include: ../../.fragments/docs/syntax/module-protect.md -->

Importarlo desde fuera de esa carpeta es un error de compilación, y el mensaje nombra el símbolo, la carpeta a la que está limitado y las dos salidas.

::: danger error
<!-- @include: ../../.fragments/docs/syntax/module-protect-error.md -->
:::

Una carpeta anidada es un límite nuevo y más estrecho: un nombre protegido en `src/auth/session/` no es visible en `src/auth/`, ni siquiera a través de un archivo barril intermedio. Una reexportación lleva el ámbito original sin modificarlo, por lo que `export *` y `export { nombre } from` no pueden ampliarlo, y una importación por namespace simplemente omite los nombres protegidos. `protect` se rechaza en una exportación default y en una reexportación, ya que ninguna de las dos declara el nombre que publica. Coloque las pruebas que necesiten un nombre protegido dentro de la carpeta que lo posee; no hay excepción para los archivos de prueba.

El lado de la importación tiene las formas correspondientes. Una importación default vincula la exportación default del módulo de origen a cualquier nombre local, una importación por namespace vincula todas las exportaciones a la vez, y ambas pueden combinarse con importaciones nombradas en una sola declaración.

<!-- @include: ../../.fragments/docs/syntax/module-imports.md -->

`tyrus.json` puede declarar alias internos estáticos en el objeto `imports`. Cada clave comienza con `#` y asigna una carpeta dentro de la raíz del proyecto. `#` asigna la raíz del código fuente, mientras que una clave más larga como `#config` asigna una carpeta anidada. Los prefijos de alias más largos tienen precedencia sobre los más cortos. Los alias se expanden una vez en tiempo de compilación y nunca resuelven paquetes, URL ni valores de runtime.

Un namespace se resuelve en tiempo de compilación: `math.add(1, 2)` se convierte en una llamada directa a la función de ese módulo, y ningún valor de namespace existe en runtime. Nombrar un miembro que el módulo no exporta es un error de compilación, y el mensaje nombra tanto el módulo como el miembro.

Un módulo también puede reenviar las exportaciones de otro módulo sin vincularlas localmente.

<!-- @include: ../../.fragments/docs/syntax/module-reexports.md -->

::: warning `export *` nunca reenvía un default
`export *` reenvía solo exportaciones nombradas. La exportación default del módulo de origen se queda atrás. Dos exportaciones que publican el mismo nombre público son un error de compilación, no un último-gana silencioso.
:::

Las APIs nativas de runtime se importan por nombres de módulo estables `tyrus:*` cuando la superficie está implementada. Las importaciones se resuelven recursivamente en tiempo de compilación; las importaciones circulares, los nombres duplicados y las importaciones de símbolos no exportados son errores de compilación.

## Valores Predeterminados de Seguridad

- Todo statement y toda declaración terminan con `;` (`E3218`), incluyendo un `const` asociado a una arrow function y una declaración con destructuring. Un constructo que se cierra solo no necesita `;`: `if`, `else`, `while`, `do ... while`, `for`, `for ... of`, `for ... in`, `match`, `switch`, `loop`, `try`/`catch`/`finally`, un bloque `defer`, `interface`, una behavior `interface`, `function` y `enum`. La cláusula de actualización de un `for` clásico, escrita `i++`, la cierra el `)`.
- Las variables deben inicializarse antes del uso.
- La ausencia se representa explícitamente con `None` y con tipos opcionales como `string?`; `null` nunca es un valor.
- Los valores non-Copy siguen reglas de ownership y move.

## Ejecución Nativa y Cobertura del Lenguaje

- APIs de runtime como filesystem y HTTP son recursos nativos enlazados al programa generado.
- Los valores opcionales usan la sintaxis familiar `T?`, `None`, `??` y narrowing; `null` no es un valor y `Optional<T>` es la forma equivalente completa.
- Algunas sintaxis se parsean antes de tener lowering completo a código nativo; las páginas experimentales lo indican explícitamente.
