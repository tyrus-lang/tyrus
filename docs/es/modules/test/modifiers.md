---
title: Modificadores
description: Omite, enfoca y planifica pruebas.
---

# Modificadores

<SignatureBlock label="Firma">test.skip(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Firma">test.only(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Firma">test.todo(name: string): void</SignatureBlock>

<SignatureBlock label="Firma">describe.skip(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Firma">describe.only(name: string, callback: () =&gt; void): void</SignatureBlock>

`test.skip` lista una prueba como omitida sin ejecutarla; su callback aún debe compilar. `describe.skip` omite todas las pruebas de la suite, y los hooks de la suite no se ejecutan.

`test.todo(name)` lista una prueba que aún no se ha escrito. Las pruebas omitidas y los todos se cuentan por separado en el resumen.

`it` acepta los mismos modificadores, al igual que un import con alias, por ejemplo `import { test as check } from "tyrus:test"`. `test.concurrent` y `test.fails` aún no están implementados y se informan como errores de compilación.

<!-- @include: ../../../.fragments/modules/test/modifiers.md -->

## Enfoque con `.only`

Cuando alguna prueba o suite de un programa está enfocada con `.only`, solo se ejecutan las pruebas enfocadas y las demás cuentan como omitidas. `.skip` prevalece sobre `.only`: enfocar nunca ejecuta una prueba que se omitió a propósito.

<!-- @include: ../../../.fragments/modules/test/modifiers-only.md -->

## Una prueba por caso

`test.each(name, cases, callback)` registra una prueba por caso, con los nombres `name [0]`, `name [1]`, y así sucesivamente. El callback recibe un caso; escrito en línea, el tipo de su parámetro se infiere de los casos. Los casos deben ser un literal de array, porque se despliegan al compilar el programa.

<!-- @include: ../../../.fragments/modules/test/each.md -->
