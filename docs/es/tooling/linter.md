---
title: Linter
description: Referencia completa de comandos, configuración, reglas y fixes de tyrus-lint.
---

# Linter

`tyrus lint` reporta diagnósticos de estilo, corrección, seguridad, performance y arquitectura separados de los errores del compilador. El linter lee `tyrus-lint.json` en la raíz del proyecto cuando existe; las reglas omitidas usan su severidad default interna.

## Comandos

<!-- @include: ../../.fragments/tooling/commands.md -->

`--fix` aplica fixes seguros y determinísticos y luego ejecuta lint de nuevo. `--format json` emite reportes legibles por máquina. `--max-warnings 0` es útil para CI. `--explain <rule>` imprime la documentación de la regla en el terminal. `--init` crea o actualiza `tyrus-lint.json` sin cambiar severidades configuradas. `--default` redefine el archivo a los defaults internos.

## Configuración

El archivo es un objeto JSON plano que mapea nombres de reglas a severidades. No existe `extends`, `presets` ni wrapper `rules`.

<!-- @include: ../../.fragments/tooling/config.md -->

| Severidad | Significado |
|-----------|-------------|
| `off` | Regla desactivada. |
| `info` | Diagnóstico informativo. |
| `warn` | Warning; no falla CI por default. |
| `error` | Error; termina con código `1`. |

## Reglas

| Regla | Default | Categoría | Autofix | Resumen |
|-------|---------|-----------|---------|---------|
| `api-no-verb-in-path` | `warn` | style | no | Avisa cuando paths de ruta usan verbos de acción en lugar de paths orientados a recurso. |
| `api-prefer-resource-nouns` | `warn` | style | no | Avisa cuando paths de ruta no parecen sustantivos de recurso. |
| `controller-must-send-response` | `error` | correctness | no | Exige que todo camino exitoso del controller envíe una respuesta. |
| `controller-no-db-access` | `error` | safety | no | Prohíbe acceso directo a la base de datos en controllers. |
| `controller-no-try-catch` | `warn` | safety | no | Avisa cuando controllers capturan errores esperados en lugar de delegar el mapeo. |
| `db-explicit-select-columns` | `info` | style | yes | Prefiere proyecciones explícitas en `SELECT` y expande campos de la interfaz de resultado local. |
| `db-no-extra-select-columns` | `info` | performance | no | Advierte cuando una consulta tipada selecciona columnas que no están declaradas en la interfaz de resultado. |
| `exhaustive-match` | `error` | correctness | no | Exige que `match` sobre tipos cerrados cubra todos los casos. |
| `http-no-raw-status-code` | `warn` | safety | no | Sugiere `HttpStatus` en lugar de códigos numéricos crudos en respuestas. |
| `no-duplicate-case` | `error` | correctness | no | Detecta cases duplicados en `switch` o `match`. |
| `no-duplicate-imports` | `error` | correctness | no | Detecta múltiples declaraciones de import del mismo módulo. |
| `no-empty-block` | `warn` | style | no | Detecta bloques vacíos sin comentario explicativo. |
| `no-empty-catch` | `warn` | safety | no | Detecta bloques `catch` vacíos sin comentario explicativo. |
| `no-floating-enum` | `error` | correctness | no | Prohíbe valores `f64`/`f128` en enums. |
| `no-force-unwrap` | `error` | safety | no | Prohíbe unwrap forzado de optional con `expr!`. |
| `no-heterogeneous-enum` | `error` | correctness | no | Prohíbe mezclar tipos primitivos de valor en un mismo enum. |
| `no-implicit-enum-primitive-cast` | `error` | safety | no | Exige casts `as` explícitos de enum a primitivos. |
| `no-implicit-heavy-move` | `warn` | performance | no | Avisa sobre moves sorprendentes de valores grandes o non-Copy. |
| `no-layer-skip` | `warn` | safety | no | Avisa cuando código API salta la capa route-controller-service-repository. |
| `no-null-literal` | `error` | correctness | no | Prohíbe la palabra clave `null`; usa `T?`, campos opcionales y `??`. |
| `no-shadow` | `warn` | style | no | Detecta bindings que sombrean bindings externos. |
| `no-string-concat-in-loop` | `warn` | performance | no | Detecta concatenación repetida de string dentro de loops. |
| `no-uninitialized-variable` | `warn` | correctness | no | Expone diagnósticos de definite assignment para `let x: T;`. |
| `no-unnecessary-allocation` | `info` | performance | no | Detecta asignaciones claramente innecesarias. |
| `no-unnecessary-await` | `warn` | style | yes | Detecta y corrige delegación redundante con `return await promise`. |
| `no-unnecessary-clone` | `warn` | performance | no | Detecta clones que podrían reemplazarse por borrows. |
| `no-upward-imports` | `error` | safety | no | Prohíbe que capas inferiores de API importen capas superiores. |
| `no-unreachable-code` | `error` | correctness | no | Detecta código después de `return`, `break`, `continue` o `throw`. |
| `no-unsafe-finally` | `error` | safety | no | Detecta cambios de flujo dentro de `finally`. |
| `no-useless-null-check` | `warn` | correctness | no | Detecta checks de null en valores que nunca pueden ser null. |
| `no-unused-functions` | `warn` | correctness | no | Detecta funciones no exportadas que nunca se llaman. |
| `no-unused-imports` | `warn` | correctness | yes | Detecta y puede eliminar specifiers de import no usados. |
| `no-unused-vars` | `warn` | correctness | no | Detecta variables no usadas; usa prefijo `_` cuando sea intencional. |
| `prefer-borrow` | `info` | performance | no | Sugiere pasar referencias cuando ownership no es necesario. |
| `prefer-const` | `warn` | style | yes | Sugiere `const` para bindings que nunca se reasignan. |
| `prefer-nullish-coalescing` | `info` | style | no | Sugiere `??` para patrones de fallback nullable. |
| `repository-no-http-error` | `warn` | safety | no | Avisa cuando repositories dependen de `HttpReply`. |
| `repository-only-db-access` | `error` | safety | no | Restringe acceso directo a la base de datos a módulos repository. |
| `route-no-business-logic` | `warn` | complexity | no | Avisa cuando módulos de ruta contienen lógica de negocio. |
| `service-no-reply` | `error` | safety | no | Prohíbe que services dependan de `HttpReply`. |
| `use-caught-error` | `warn` | safety | no | Detecta errores capturados que nunca se usan. |

## Match exhaustivo

`exhaustive-match` exige que un `match` sobre un tipo cerrado, una tagged union, una union literal o un booleano, cubra todos los casos. Es aplicado por el compilador como `E0004`, así que reporta incluso con la regla de lint deshabilitada.

<!-- @include: ../../.fragments/tooling/exhaustive-match.md -->

## Código nullable

tyrus no usa el literal `null` en código normal. Expresa ausencia con `T?`, campos opcionales, parámetros opcionales y `??`.

<!-- @include: ../../.fragments/tooling/nullable-code.md -->

## Arquitectura de API

Los proyectos creados al elegir el target API en las preguntas de `tyrus new` reciben defaults de lint orientados a API. Los módulos de ruta declaran rutas, los controllers traducen fronteras HTTP, los services contienen comportamiento de negocio y los repositories concentran acceso directo a la base de datos.

<!-- @include: ../../.fragments/tooling/api-architecture.md -->

## Quick Fixes

Las reglas exponen Quick Fixes cuando la edición es determinística. `prefer-const` puede reescribir `let` a `const`; `no-unused-imports` puede eliminar specifiers de import no usados. Las reglas que exigen una decisión de diseño, como renombrar una variable sombreada, reportan ayuda pero no inventan una edición.

<!-- @include: ../../.fragments/tooling/quick-fixes.md -->

::: warning Las reglas de arquitectura intencionalmente no tienen autofix
Reglas como `controller-no-db-access` y `service-no-reply` identifican problemas de diseño. Exigen mover código a la capa correcta, no una edición mecánica.
:::
