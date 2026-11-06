---
title: Playground
description: Escriba, compile y ejecute código tyrus directamente en el navegador.
---

# Playground

El Tyrus Playground es un editor web para escribir, compilar y ejecutar código tyrus sin necesidad de instalar nada localmente. Compila tu código a WebAssembly en el servidor y lo ejecuta en un Web Worker con sandbox en el navegador.

## Acceso

Abre [playground.tyruslang.org](https://playground.tyruslang.org) en cualquier navegador moderno.

## Funcionalidades

- **Monaco Editor** con resaltado de sintaxis personalizado para Tyrus.
- **Temas oscuro y claro** mediante el botón de alternancia.
- **Ejemplos precargados** (Hello World, Fibonacci, Match, Loops, Const, Enums).
- **Ctrl+Enter** para compilar y ejecutar.
- **Panel de salida** que muestra stdout, stderr y errores de compilación.
- **Enlaces de compartición** mediante URLs cortas (`/s/{shortId}`).

## Cómo Funciona

1. Escribes código en el editor.
2. Hacer clic en **Run** envía el código al backend.
3. El backend lo compila a WebAssembly usando `tyrus build --compile-target wasm`.
4. El binario WASM se carga en un Web Worker con sandbox.
5. El Worker ejecuta el código y captura stdout/stderr.
6. La salida se muestra en el panel de salida.

## Qué se ejecuta en el sandbox

El playground ejecuta tu programa en el navegador, así que todo lo que necesite un sistema operativo detrás no puede funcionar ahí. El resto sí funciona: el lenguaje en sí, la biblioteca estándar y las partes del runtime que son cómputo puro.

Disponible: cadenas y expresiones regulares, arreglos y sus métodos funcionales, `Map` y `Set`, `decimal`, JSON, matemáticas, validadores y `tyrus:temporal`, incluida la conversión de zona horaria.

::: warning `tyrus:cli`, `tyrus:crypto`, `tyrus:fs`, `tyrus:http`, `tyrus:net`, `tyrus:os`, `tyrus:process`, `tyrus:stream` y `tyrus:task` no corren en el playground
Cada uno de estos necesita acceso real al sistema operativo que el sandbox del navegador no tiene, un pool de threads para el paralelismo de `tyrus:task`, el sistema de archivos para `tyrus:fs`, sockets para `tyrus:net`/`tyrus:http`, y así sucesivamente. Importar cualquiera de estos se rechaza al compilar el programa, nombrando el módulo (`module `tyrus:task` is not available in a sandboxed build`, por ejemplo). El namespace ambiental `process` no tiene import que rechazar, así que se declara no disponible cuando el programa lo usa. El mismo rechazo vale para cualquier build cuyo destino sea WebAssembly, no solo la del playground, porque quien no tiene el acceso es el destino y no el sandbox; sin ella el programa llegaba al enlazador y fallaba allí con un símbolo de runtime indefinido en vez de un mensaje que nombrara el import. Ejecuta ejemplos que usen cualquiera de estos localmente con `tyrus run`.
:::

Usar una API ambiental que el sandbox no provee no hace fallar el programa en silencio. Según la API, lanza un error capturable cuyo `code` sigue el formato `ERR_<MÓDULO>_UNAVAILABLE`, o detiene el programa con un mensaje que nombra lo que se intentó usar. En ambos casos el motivo aparece en el panel de salida.

<!-- @include: ../../.fragments/tooling/playground/what-runs-in-the-sandbox.md -->

<!-- @include: ../../.fragments/tooling/playground/what-runs-in-the-sandbox-2.md -->

::: warning `async`/`await` no se ejecuta en el playground
El código asíncrono compila, pero no hay nada que lo impulse: el sandbox no tiene ejecutor de tareas y `main()` corre hasta el final de forma síncrona. Un programa que usa `await` no producirá el resultado esperado. Ejecútalo localmente con `tyrus run`.

Esto es una propiedad del sandbox, no una carencia del lenguaje: el código asíncrono funciona normalmente fuera del playground.
:::

## Limitaciones

- El código está limitado a 8 KB.
- La ejecución está limitada a 5 segundos.
- La memoria WASM está limitada a 8 MB.
- Los módulos de sistema de archivos, red, SO y la ejecución de `async`/`await` no están disponibles en el sandbox.
- La compilación tiene un límite de 5 solicitudes por minuto por IP.
