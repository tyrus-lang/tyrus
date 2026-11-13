---
title: Errores y diagnósticos
description: Diagnósticos de compilador, linter, runtime y editor con ejemplos.
---

# Errores y diagnósticos

Los diagnósticos de tyrus usan códigos estables, rangos de origen precisos, labels relacionados y ayuda accionable cuando existe una corrección concreta. El mismo análisis del compilador alimenta la CLI y el Language Server.

::: tip Lee el rango primero
Los diagnósticos deben apuntar al token, expresión, declaración o inicializador que causó el problema. Cuando el editor muestra un squiggle, empieza por el rango resaltado.
:::

## Errores del compilador

Los errores de tipo detienen la compilación antes de la generación de código LLVM.

::: danger error
<!-- @include: ../../.fragments/language/errors/compiler-error-code.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/errors/compiler-error-output.md -->
:::

## Errores de borrow y move

Los valores non-Copy se mueven por default. Después de un move, el binding anterior queda inválido.

::: danger error
<!-- @include: ../../.fragments/language/errors/borrow-move-code.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/errors/borrow-move-output.md -->
:::

::: warning Ownership se aplica antes del runtime
Los errores de move y borrow son fallas de compilación. El binario generado nunca necesita un garbage collector para recuperarse de estos errores.
:::

## Warnings del linter

Los warnings de lint reportan código válido, pero riesgoso, ruidoso o inconsistente con la arquitectura del proyecto.

::: danger error
<!-- @include: ../../.fragments/language/errors/linter-warning-code.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/errors/linter-warning-output.md -->
:::

Cuando la corrección es determinística, el Language Server expone la misma edición como Quick Fix.

## Fallas de runtime

Los programas generados escriben fallas de runtime en stderr y terminan con status distinto de cero. La salida normal de `print` permanece en stdout.

::: danger error
<!-- @include: ../../.fragments/language/errors/runtime-failure-code.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/errors/runtime-failure-output.md -->
:::

Las fallas de runtime incluyen comprobaciones de seguridad insertadas por el compilador, como overflow, división por cero, narrowing inválido, unwrap forzado de optional y fallas de validación.

## Fallas de validación

Validator decorations pueden fallar en tiempo de compilación para literales y en runtime para valores dinámicos.

::: danger error
<!-- @include: ../../.fragments/language/errors/validation-failure-code.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/errors/validation-failure-output.md -->
:::

::: danger Aborts de validación usan stderr
Las fallas de validación en runtime llaman al helper de panic de validación del runtime, imprimen el diagnóstico en stderr y terminan el proceso con status distinto de cero.
:::

## Diagnósticos en el editor

VS Code, Neovim, Zed e IntelliJ/WebStorm deben recibir diagnósticos desde `tyrus lsp`. Los clientes específicos de editor no deben reimplementar reglas de tipo.

## Códigos de error

La tabla lista todos los diagnósticos públicos del compilador con el formato `E####`. Usa `tyrus explain <código>` para una referencia concisa en la terminal; los mensajes, labels y Quick Fixes disponibles siguen dependiendo del contexto.

<!-- @include: ../../.fragments/language/errors/error-codes.md -->
