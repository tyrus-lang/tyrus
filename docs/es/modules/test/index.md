---
title: Test
description: Suites de prueba, aserciones, hooks y modificadores ejecutados por `tyrus test`.
---

# Test

`tyrus:test` es el framework de pruebas incluido en el lenguaje. Las suites se escriben con `describe`, las pruebas con `test` o su alias `it`, y las comprobaciones con `expect`. `tyrus test` descubre los archivos de prueba, compila cada uno en su propio binario e informa todos los resultados.

<!-- @include: ../../../.fragments/modules/test/describe.md -->

Todo nombre se importa explícitamente, como en cualquier otro módulo nativo. Un archivo de prueba es cualquier archivo cuyo nombre termina en `.test.tyr`; `__tests__/` es la carpeta convencional para ellos, pero no obligatoria.

## API

- [describe](./describe): agrupa pruebas en una suite con nombre; las suites se pueden anidar.
- [test / it](./test): registra una prueba con nombre.
- [expect](./expect): comprueba un valor con un matcher, invertido con `.not`.
- [Hooks](./hooks): `beforeAll`, `afterAll`, `beforeEach` y `afterEach`.
- [Modificadores](./modifiers): `.skip`, `.only` y `.todo` en pruebas y suites.
- [Ejecución de pruebas](./running): descubrimiento, opciones de línea de comandos y el informe.
