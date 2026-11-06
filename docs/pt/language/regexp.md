---
title: Regex
description: Expressões regulares nativas, literais, flags, métodos, integração com string, validators e limitações.
---

# Regex

`Regex` é um tipo nativo do tyrus. Ele representa uma expressão regular compilada e é escrito com um literal `/padrão/flags`.

## Construção

<!-- @include: ../../.fragments/language/regexp/construction.md -->

Padrões e flags são validados durante a checagem de tipos. `new` não faz parte da construção de `Regex`.

## Flags

As flags suportadas são `i` para correspondência sem distinção entre maiúsculas e minúsculas, `m` para âncoras multilinha, `s` para dot-all e `x` para o modo free-spacing. As flags `g`, `u`, `y` e `d` não são suportadas.

## Métodos

#### `test()`

Retorna se o padrão corresponde em qualquer posição de uma string.

<SignatureBlock label="Assinatura">test(str: string): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/method-test.md -->

#### `exec()`

Retorna a primeira correspondência e suas capturas, ou um valor ausente quando não há correspondência.

<SignatureBlock label="Assinatura">exec(str: string): string[]?</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/method-exec.md -->

#### `execAll()`

Retorna todas as correspondências não sobrepostas e suas capturas, ou um valor ausente quando não há correspondência.

<SignatureBlock label="Assinatura">execAll(str: string): string[][]?</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/method-exec-all.md -->

#### `toString()`

Retorna o padrão delimitado por barras e as flags ativas.

<SignatureBlock label="Assinatura">toString(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/method-to-string.md -->

## Métodos de `String` com `Regex`

#### `match()`

Retorna a primeira correspondência e suas capturas, ou um valor ausente quando a string não corresponde.

<SignatureBlock label="Assinatura">match(re: Regex): string[]?</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/string-match.md -->

#### `matchAll()`

Retorna todas as correspondências não sobrepostas e suas capturas, ou um valor ausente quando a string não corresponde.

<SignatureBlock label="Assinatura">matchAll(re: Regex): string[][]?</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/string-match-all.md -->

#### `search()`

Retorna o índice inicial da primeira correspondência, ou `-1` quando a string não corresponde.

<SignatureBlock label="Assinatura">search(re: Regex): i32</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/string-search.md -->

#### `replace()`

Substitui a primeira correspondência pela string de substituição.

<SignatureBlock label="Assinatura">replace(re: Regex, replacement: string): string</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/string-replace.md -->

#### `split()`

Divide a string em cada correspondência não sobreposta.

<SignatureBlock label="Assinatura">split(re: Regex): string[]</SignatureBlock>

<!-- @include: ../../.fragments/language/regexp/string-split.md -->

## Limitações

Valores `Regex` são imutáveis. Não existe `lastIndex` mutável, nem flag `g`, nem hooks de símbolos well-known, nem objeto de resultado de correspondência, nem look-around ou backreferences.
