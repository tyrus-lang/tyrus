---
title: test / it
description: Registra una prueba con nombre.
---

# test / it

<SignatureBlock label="Firma">test(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Firma">it(name: string, callback: () =&gt; void): void</SignatureBlock>

<SignatureBlock label="Firma">test(name: string, callback: () =&gt; Promise&lt;void&gt;): void</SignatureBlock>

`test` e `it` son la misma función con dos nombres. El callback se ejecuta una vez. La prueba pasa cuando el callback retorna y falla cuando una aserción falla o el callback lanza; el mensaje de fallo se imprime debajo del nombre de la prueba.

Un callback no puede asignar a un binding declarado fuera de él, porque el callback sobrevive al ámbito que declaró el binding (`E0436`). Mantén la preparación que necesita una prueba dentro de esa prueba, o en un `beforeEach` que prepare un estado que la prueba lee mediante una función.

<!-- @include: ../../../.fragments/modules/test/test.md -->

## Pruebas asíncronas

Un callback `async` se espera antes de que se ejecute el siguiente hook o prueba. Una aserción que falla después de un `await` reprueba la prueba como cualquier otra, al igual que una promise rechazada.

<!-- @include: ../../../.fragments/modules/test/test-async.md -->
