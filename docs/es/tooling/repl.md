---
title: REPL
description: Consola interactiva para explorar, prototipar y aprender tyrus.
---

# REPL

REPL significa **Read-Eval-Print Loop**: un programa que lee una entrada, la evalúa, imprime el resultado y repite.

`tyrus repl` inicia una consola interactiva donde escribes y ejecutas código tyrus línea por línea. Es útil para explorar APIs, probar expresiones y aprender el lenguaje sin crear un proyecto.

## Inicia el REPL

<!-- @include: ../../.fragments/tooling/repl-start.md -->

La transcripción anterior muestra el banner actual de inicio y la salida exacta de `:help`.

El REPL requiere un terminal interactivo. Se niega a ejecutarse en un entorno no TTY, como un pipe.

## Características

### Resaltado de sintaxis

El código se resalta en tiempo real mientras escribes, reflejando el esquema de colores clásico de Monokai: keywords de control, operadores y la mayoría de las demás keywords en rosa/rojo, keywords de almacenamiento/tipo (`const`, `let`, tipos primitivos, `interface`/`enum`/`type`, `=>`) en cian, strings y template literals en amarillo, números y booleanos en morado, llamadas a función en verde, identificadores en blanco grisáceo y comentarios en gris.

### Auto-completado

Presiona Tab para completar comandos, keywords, tipos primitivos y símbolos definidos en la sesión actual. Los comandos se completan después de escribir `:` (ej.: `:lo` → `:load `). Las rutas de archivo se completan para `:load` y `:restore`; `:restore` también completa los nombres de sesiones guardadas anteriormente. En acceso a miembros, los miembros se completan después de `.` o `?.`: escribir `console.` sugiere `log`, `error` y `warn`, y el nombre de un enum de la sesión o de un import nativo (ej.: `HttpStatus.`) sugiere sus miembros.

### Entrada multilínea

El REPL detecta entrada incompleta con el parser. Cuando una línea termina a mitad de una construcción (por ejemplo un `{`, `[` o `(` sin cierre), el prompt cambia a `.... ` para continuación. Un error de parse real, como un `;` suelto dentro de una llamada sin cerrar, se informa de inmediato, sin esperar a más líneas. Presiona Enter en una línea vacía para evaluar la entrada acumulada.

### Historial de comandos

Los comandos anteriores se guardan en `~/.tyrus_repl_history` (hasta 5000 entradas) y están disponibles con las flechas Up/Down. El archivo de historial persiste entre sesiones.

## Comandos

Todo metacomando comienza con `:`.

| Comando | Finalidad |
|---------|-----------|
| `:help` | Muestra comandos disponibles |
| `:clear` | Limpia la pantalla y el buffer de la sesión |
| `:load <file>` | Carga y ejecuta un archivo `.tyr` |
| `:list` | Lista sesiones guardadas vía `:save` |
| `:type <expr>` | Muestra el tipo inferido de una expresión |
| `:doc <symbol>` | Muestra documentación de un símbolo |
| `:edit` | Edita el buffer de la sesión en el editor externo (`$EDITOR`) |
| `:save <name.tyr>` | Guarda el buffer de la sesión en `~/.tyrus/repl/<name>.tyr` |
| `:restore <name\|file> [--merge\|-m]` | Restaura una sesión guardada o un archivo, reiniciando la sesión antes (valida el código) |
| `:delete <name>` | Elimina una sesión guardada de `~/.tyrus/repl` |
| `:reset` | Reinicia la sesión completa |
| `:exit` | Sale del REPL |
| `:quit` | Alias para `:exit` |
| `:q` | Alias para `:exit` |

## Gestión de sesión

El REPL preserva el estado de la sesión entre comandos. Las variables, funciones y alias de tipo definidos durante la sesión permanecen disponibles a medida que ingresas nuevo código.

Las sesiones son archivos `.tyr` guardados en `~/.tyrus/repl/`, uno por sesión guardada, por lo que sobreviven entre invocaciones del REPL y son independientes del directorio actual. `:save <name.tyr>` escribe el buffer de la sesión en `~/.tyrus/repl/<name>.tyr` y exige la extensión `.tyr`; `:restore <name>` recarga una sesión guardada (la extensión es opcional, y una ruta de archivo existente también funciona) y la reproduce tal como se escribió: cada sentencia se repite como una transcripción `tyrus> ` con syntax highlighting y luego se ejecuta, de modo que cada salida del programa aparece justo debajo de la línea que la produjo. Por defecto `:restore` reinicia la sesión antes, de modo que el código restaurado siempre se ejecuta como estaba al momento del `:save`; pasa `--merge` (o `-m`) para mantener el buffer actual y fusionar el código restaurado en él. Con `--merge`, un nombre ya definido en cualquiera de los dos lados aborta el restore con un error antes de que se ejecute nada, dejando la sesión sin cambios, resuelve el conflicto con `:reset` e inténtalo de nuevo. `:delete <name>` elimina una sesión guardada (la extensión también es opcional). `:list` muestra las sesiones guardadas con su cantidad de líneas, tamaño y cuándo se guardaron, de la más reciente a la más antigua. Si una unidad restaurada contiene código inválido, el restore se rechaza y la sesión no se modifica.

`:load <file>` reproduce un archivo del proyecto en la sesión actual del mismo modo que `:restore`: cada sentencia se repite como una transcripción `tyrus> ` y luego se ejecuta, en lugar de solo reportar una cantidad de líneas.

`:reset` limpia el buffer y vuelve al estado inicial. `:clear` limpia la pantalla sin descartar el buffer.

## Inspección de tipos

El comando `:type` muestra el tipo inferido de cualquier expresión sin evaluarla:

<!-- @include: ../../.fragments/tooling/repl/type-inspection.md -->

## Consulta de documentación

El comando `:doc` muestra documentación de símbolos definidos en la sesión, incluyendo nombres de parámetros, tipos de retorno, anotaciones `@param`, `@returns` y `@deprecated`, y ejemplos de uso:

<!-- @include: ../../.fragments/tooling/repl/documentation-lookup.md -->

## Editor externo

El comando `:edit` abre el buffer de la sesión en el editor especificado por `$EDITOR` o `$VISUAL` (predeterminado: `nano`). Después de que el editor salga, el buffer modificado se compila, si hay errores, la sesión revierte al estado anterior.

## Compilación

Cada entrada se compila desde cero usando el compilador JIT LLVM en proceso, lo que hace que la evaluación sea instantánea. Cuando el JIT no está disponible (sin runtime LLVM), el REPL cae al modo compilado mediante clang.

El buffer de la sesión se envuelve en una función `main` (`const main = (): void => { ... };`) para que las declaraciones de función y los bindings de variables coexistan en el mismo ámbito.
