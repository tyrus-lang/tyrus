---
title: describe
description: Agrupa pruebas en una suite con nombre.
---

# describe

<SignatureBlock label="Firma">describe(name: string, callback: () =&gt; void): void</SignatureBlock>

`describe(name, callback)` abre una suite y ejecuta `callback` de inmediato, así que las pruebas y los hooks registrados dentro pertenecen a esa suite. Ninguna prueba se ejecuta durante el registro: las pruebas se ejecutan después de que el programa lo ha registrado todo, en el orden en que se escribieron, y una suite anidada se ejecuta donde aparece.

El nombre completo de una prueba es su ruta por las suites, unida con ` > `, como en `discount > from 100 on > is ten percent`.

Cuando una aserción falla mientras se registra el callback de una suite, la suite se informa como fallida y ninguna de sus pruebas se ejecuta.

<!-- @include: ../../../.fragments/modules/test/describe.md -->
