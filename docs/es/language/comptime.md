---
title: Evaluación en Tiempo de Compilación
description: Evalúa expresiones deterministas y construye constantes durante la compilación con comptime.
---

# Evaluación en Tiempo de Compilación

`comptime(expresión)` evalúa una expresión determinista durante la compilación y reemplaza el wrapper por un literal común en el programa generado. Puede evaluar literals directamente o llamar una `comptime function` o arrow `comptime const` explícitamente elegible. La misma función cualificada sigue pudiendo llamarse normalmente en runtime.

## Precalcula Fibonacci

Este ejemplo calcula el número de Fibonacci en la posición 50 durante la compilación. La implementación iterativa termina en 50 pasos, mientras que una versión recursiva ingenua repetiría trabajo exponencialmente y podría superar el presupuesto determinista de evaluación. El resultado usa `i64` porque no cabe en `i32`.

<!-- @include: ../../.fragments/language/comptime/fibonacci.md -->

El loop y las mutaciones intermedias se ejecutan solo en el compilador. En runtime, `fibonacci50` ya es el literal `12586269025`. Llamar `fibonacci(...)` fuera de `comptime(...)` sigue siendo válido y se ejecuta normalmente en runtime.

## Funciones Elegibles

El destino de una llamada en tiempo de compilación debe usar `comptime function nombre(...)` o `comptime const nombre = (...) => ...`. El código elegible puede usar bindings locales, condicionales, loops, recursión, `break`, `continue`, arrays, tuples, records, enums y aritmética determinista. No puede leer estado externo mutable, llamar una función común o nativa, realizar I/O, usar tiempo o aleatoriedad, lanzar o capturar errores, usar `async`/`await` ni crear un valor con identidad de runtime. Una arrow de compilación solo puede capturar constantes que también puedan evaluarse durante la compilación.

## Valores y Límites

Los resultados de compilación pueden ser booleans, enteros, valores de punto flotante compatibles, decimals, chars, strings, `void`, arrays, tuples, records y enums cuyo contenido también sea un valor de compilación. Closures, valores de función, referencias, promises, regexes, `Map`, `Set` y recursos nativos se rechazan. Cada wrapper tiene límites deterministas de 1.000.000 de pasos de evaluación, 256 llamadas anidadas, 65.536 elementos estructurales y 16 MiB de valores temporales y materializados.

## Feedback en el Editor

Pasa el cursor sobre la keyword `comptime` para ver el tipo evaluado y un resumen limitado del valor. Los fallos señalan su causa exacta e incluyen el wrapper y el rastro de llamadas como información relacionada. Cuando una expresión es válida en runtime pero no es elegible para evaluación durante la compilación, el editor puede eliminar el wrapper con el Quick Fix `Evaluate this expression at runtime`.
