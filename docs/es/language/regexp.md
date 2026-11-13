---
title: Regex
description: Expresiones regulares nativas, literales, flags, métodos, integración con string, validators y limitaciones.
---

# Regex

`Regex` es un tipo nativo de tyrus. Representa una expresión regular compilada y se escribe con un literal `/patrón/flags`.

## Construcción

<!-- @include: ../../.fragments/language/regexp/construction.md -->

Los patrones y las flags se validan durante la comprobación de tipos. `new` no forma parte de la construcción de `Regex`.

## Flags

Las flags admitidas son `i` para coincidencia sin distinción entre mayúsculas y minúsculas, `m` para anclas multilínea, `s` para dot-all y `x` para el modo free-spacing. Las flags `g`, `u`, `y` y `d` no se admiten.

## Métodos

#### `test()`

Devuelve si el patrón coincide en cualquier posición de una string.

<SignatureBlock label="Firma">test(str: string): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/method-test.md -->

#### `exec()`

Devuelve la primera coincidencia y sus capturas, o un valor ausente cuando no hay coincidencia.

<SignatureBlock label="Firma">exec(str: string): string[]?</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/method-exec.md -->

#### `execAll()`

Devuelve todas las coincidencias no superpuestas y sus capturas, o un valor ausente cuando no hay coincidencia.

<SignatureBlock label="Firma">execAll(str: string): string[][]?</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/method-exec-all.md -->

#### `toString()`

Devuelve el patrón delimitado por barras y las flags activas.

<SignatureBlock label="Firma">toString(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/method-to-string.md -->

## Métodos de `String` con `Regex`

#### `match()`

Devuelve la primera coincidencia y sus capturas, o un valor ausente cuando la string no coincide.

<SignatureBlock label="Firma">match(re: Regex): string[]?</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/string-match.md -->

#### `matchAll()`

Devuelve todas las coincidencias no superpuestas y sus capturas, o un valor ausente cuando la string no coincide.

<SignatureBlock label="Firma">matchAll(re: Regex): string[][]?</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/string-match-all.md -->

#### `search()`

Devuelve el índice inicial de la primera coincidencia, o `-1` cuando la string no coincide.

<SignatureBlock label="Firma">search(re: Regex): i32</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/string-search.md -->

#### `replace()`

Reemplaza la primera coincidencia con la string de reemplazo.

<SignatureBlock label="Firma">replace(re: Regex, replacement: string): string</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/string-replace.md -->

#### `split()`

Divide la string en cada coincidencia no superpuesta.

<SignatureBlock label="Firma">split(re: Regex): string[]</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/string-split.md -->

## Limitaciones

Los valores `Regex` son inmutables. No existe `lastIndex` mutable, ni flag `g`, ni hooks de símbolos well-known, ni objeto de resultado de coincidencia, ni look-around o backreferences.
