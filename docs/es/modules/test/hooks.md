---
title: Hooks
description: Ejecuta preparación y limpieza alrededor de las pruebas.
---

# Hooks

<SignatureBlock label="Firma">beforeAll(callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Firma">afterAll(callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Firma">beforeEach(callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Firma">afterEach(callback: () =&gt; void): void</SignatureBlock>

`beforeAll` se ejecuta una vez antes de la primera prueba de su suite y `afterAll` una vez después de la última. `beforeEach` y `afterEach` se ejecutan alrededor de cada prueba de su suite y de todas las suites anidadas: `beforeEach` desde la suite más externa hacia dentro, y `afterEach` en el orden inverso. Los hooks registrados fuera de cualquier `describe` se aplican a todo el archivo.

`afterEach` se ejecuta aunque la prueba haya fallado, así que los recursos adquiridos en la preparación se liberan. Una suite sin ninguna prueba que ejecutar, por `.skip` o `.only`, no ejecuta sus hooks.

Un `beforeAll` que falla reprueba todas las pruebas de su suite sin ejecutarlas. Un `beforeEach` que falla reprueba su prueba, y un `afterEach` que falla se añade al informe de la prueba tras la cual se ejecutó.

Los hooks también pueden ser `async`; cada uno se espera antes de la prueba que prepara.

<!-- @include: ../../../.fragments/modules/test/hooks.md -->
