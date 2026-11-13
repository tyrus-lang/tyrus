---
title: Regex
description: Native regular expressions, literals, flags, methods, string integration, validators, and limitations.
---

# Regex

`Regex` is a native tyrus type. It represents a compiled regular expression and is written with a `/pattern/flags` literal.

## Construction

<!-- @include: ../.fragments/language/regexp/construction.md -->

Patterns and flags are validated during type checking. `new` is not part of Regex construction.

## Flags

Supported flags are `i` for case-insensitive matching, `m` for multiline anchors, `s` for dot-all and `x` for free-spacing mode. The flags `g`, `u`, `y` and `d` are not supported.

## Methods

#### `test()`

Returns whether the pattern matches anywhere in a string.

<SignatureBlock label="Signature">test(str: string): boolean</SignatureBlock>

<!-- @include: ../.fragments/language/regexp/method-test.md -->

#### `exec()`

Returns the first match and its captures, or an absent value when no match exists.

<SignatureBlock label="Signature">exec(str: string): string[]?</SignatureBlock>

<!-- @include: ../.fragments/language/regexp/method-exec.md -->

#### `execAll()`

Returns all non-overlapping matches and their captures, or an absent value when no match exists.

<SignatureBlock label="Signature">execAll(str: string): string[][]?</SignatureBlock>

<!-- @include: ../.fragments/language/regexp/method-exec-all.md -->

#### `toString()`

Returns the slash-delimited pattern and active flags.

<SignatureBlock label="Signature">toString(): string</SignatureBlock>

<!-- @include: ../.fragments/language/regexp/method-to-string.md -->

## String methods with `Regex`

#### `match()`

Returns the first match and its captures, or an absent value when the string does not match.

<SignatureBlock label="Signature">match(re: Regex): string[]?</SignatureBlock>

<!-- @include: ../.fragments/language/regexp/string-match.md -->

#### `matchAll()`

Returns all non-overlapping matches and their captures, or an absent value when the string does not match.

<SignatureBlock label="Signature">matchAll(re: Regex): string[][]?</SignatureBlock>

<!-- @include: ../.fragments/language/regexp/string-match-all.md -->

#### `search()`

Returns the start index of the first match, or `-1` when the string does not match.

<SignatureBlock label="Signature">search(re: Regex): i32</SignatureBlock>

<!-- @include: ../.fragments/language/regexp/string-search.md -->

#### `replace()`

Replaces the first match with the replacement string.

<SignatureBlock label="Signature">replace(re: Regex, replacement: string): string</SignatureBlock>

<!-- @include: ../.fragments/language/regexp/string-replace.md -->

#### `split()`

Splits the string at every non-overlapping match.

<SignatureBlock label="Signature">split(re: Regex): string[]</SignatureBlock>

<!-- @include: ../.fragments/language/regexp/string-split.md -->

## Limitations

Regex values are immutable. There is no mutable `lastIndex`, no `g` flag, no well-known symbol hooks, no match-result object, and no look-around or backreferences.
