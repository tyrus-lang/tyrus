---
title: Ejecución de pruebas
description: Descubrimiento y el informe.
---

# Ejecución de pruebas

## Descubrimiento

Un archivo de prueba es cualquier archivo cuyo nombre termina en `.test.tyr`, esté donde esté en el proyecto. `__tests__/` es la carpeta convencional para ellos, pero no obligatoria: una prueba también puede estar junto al código que cubre. `tyrus test` encuentra todos los archivos de prueba, omitiendo `target/` y los directorios ocultos, compila cada uno en su propio binario y lo ejecuta en su propio proceso, así que un archivo que falla de forma abrupta no detiene a los demás, y una ejecución con más de un archivo termina con un resumen combinado. Pasa un archivo para ejecutar solo ese archivo, o un directorio para ejecutar los archivos de prueba que contiene.

Solo un archivo de prueba puede importar `tyrus:test`. Un módulo de helpers de prueba compartidos que llama a `expect` es, por lo tanto, también un archivo de prueba: nómbralo `helpers.test.tyr`, impórtalo como `#/helpers.test`, y no aporta pruebas propias. El import se rechaza en cualquier otro lugar (`E0434`), incluso en un módulo auxiliar que una prueba importe, así que el runner que el módulo arma nunca llega a un binario de producción.

Los archivos de prueba se compilan y ejecutan en paralelo, tantos a la vez como procesadores haya. La salida se agrupa por archivo y se imprime en orden de descubrimiento, sin importar en qué orden terminaron los procesos, y los colores siguen la terminal y `NO_COLOR`.

<!-- @include: ../../../.fragments/tooling/cli/test-options.md -->

## Informe

Cada archivo imprime una línea por prueba, con sangría bajo sus suites, seguida de un resumen. Una prueba que falla imprime su mensaje debajo de su nombre, y el proceso termina con estado 1 cuando alguna prueba falla.

<!-- @include: ../../../.fragments/tooling/cli/test-report.md -->
