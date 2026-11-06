---
title: Strings
description: String literals, template literals, interpolation, multiline text, and literal string types in tyrus.
---

# Strings

`string` is the native UTF-8 text type in tyrus. Source syntax is concise and statically checked, while the runtime implementation is native.

## Quoted Strings

<!-- @include: ../.fragments/language/strings/quoted-strings.md -->

The formatter normalizes quotes according to `quote_style`, unless it is set to `"off"`, which preserves the original quotes and escapes of every string literal.

## Multiline Strings

A string delimited by `"""` can span multiple lines while staying aligned with the surrounding code. The compiler dedents it at compile time: from every line, it strips as many leading spaces as the closing `"""` itself is indented with, and it drops a first line left blank right after the opening `"""`. The result is an ordinary `string` literal, so there is no runtime cost to writing an aligned query or template instead of one flush against the left margin.

<!-- @include: ../.fragments/language/strings/multiline-strings.md -->

The dedent margin only counts spaces; a `"""` closed on a tab-indented line measures a margin of zero, so use spaces for the indentation you want stripped.

## Template Literals

Use template literals for interpolation and multiline text.

<!-- @include: ../.fragments/language/strings/template-literals.md -->

An interpolation may carry a format specifier after `:`, `.Nf` fixes a float to exactly `N` decimal places, and `0Nd` zero-pads an integer to a total width of `N` characters. `.Nf` requires a float value and `0Nd` requires an integer value; a mismatch is a compile error (`E0717`) naming the required kind.

A union-typed value may be interpolated too. Which variant it holds is a run-time fact, so the rendering is chosen at run time and matches what `print` would print for that variant.

## Literal String Types

String literals can participate in literal unions and tagged unions.

A literal union is an ordinary value: compare it with any of its variants (`status === "active"`), branch on it with a `match` that needs no `_` when it covers every variant, build one with a ternary, collect them in an array, and call on it anything a `string` accepts, every variant of a string-literal union is a `string`. The comparison is a single byte check rather than a text comparison, because such a union is represented by which variant it holds and nothing else.

<!-- @include: ../.fragments/language/strings/literal-string-types.md -->

::: warning Literal values are not enum members
Use literal strings for small closed text domains. Use `enum` when the domain needs named members and native enum semantics.
:::

## Grapheme clusters

`chars()` gives Unicode scalar values; `graphemes()` gives what a reader calls characters. They differ wherever one visible character is written with several scalar values, an emoji built from a zero-width joiner sequence, or a letter followed by a combining accent.

<!-- @include: ../.fragments/language/strings/graphemes.md -->

Indexing, slicing and `length` count scalar values, not clusters: that is the unit every position in the string API is measured in, and it is what makes `text.charAt(text.length - 1)` the last one. `graphemes()` is the answer to the display question, how many characters a reader sees, and where a cursor would stop, and it returns `string[]`, because a cluster can hold more than one scalar value.

## String Properties

### length

<!-- @include: ../.fragments/language/strings/length.md -->

`length` is read from the value, not counted: a `string` carries its character count alongside its text, so asking for it costs the same whether the string holds three characters or three million. U+0000 is an ordinary character in a `string`, the text is delimited by its recorded length, not by a terminator, so `"a\u{0}b".length` is `3` and the whole of it survives comparison, interpolation and printing.

`length` counts characters, Unicode scalar values, never the underlying UTF-8 bytes, so it agrees with `chars()` on any text. Every position in the string API is measured in that same unit: `charAt()`, `charCodeAt()`, `slice()`, `substring()`, `indexOf()`, `lastIndexOf()`, `padStart()` and `padEnd()` all count characters, which is what makes `text.charAt(text.length - 1)` the last character of any string, accented or not.

## Methods

| Category | Methods |
| --- | --- |
| Case conversion | [`toUpperCase()`](#touppercase), [`toLowerCase()`](#tolowercase) |
| Case transformation | [`toSlugCase()`](#toslugcase), [`toDashCase()`](#todashcase), [`toKebabCase()`](#tokebabcase), [`toPascalCase()`](#topascalcase), [`toCamelCase()`](#tocamelcase), [`toSnakeCase()`](#tosnakecase) |
| Trimming | [`trim()`](#trim), [`trimStart()`](#trimstart), [`trimEnd()`](#trimend) |
| Searching | [`includes()`](#includes), [`startsWith()`](#startswith), [`endsWith()`](#endswith), [`indexOf()`](#indexof), [`lastIndexOf()`](#lastindexof), [`search()`](#search) |
| Slicing & extracting | [`slice()`](#slice), [`substring()`](#substring), [`charAt()`](#charat), [`charCodeAt()`](#charcodeat), [`truncate()`](#truncate), [`truncateEllipsis()`](#truncateellipsis) |
| Transforming | [`replace()`](#replace), [`repeat()`](#repeat), [`padStart()`](#padstart), [`padEnd()`](#padend), [`concat()`](#concat), [`split()`](#split) |
| HTML & encoding | [`stripHtml()`](#striphtml), [`escapeHtml()`](#escapehtml), [`urlEncode()`](#urlencode), [`urlDecode()`](#urldecode) |
| Pattern matching | [`match()`](#match), [`matchAll()`](#matchall) |
| Iteration | [`chars()`](#chars), [`graphemes()`](#graphemes) |

### Case conversion

---

#### `toUpperCase()`

Returns the string with all uppercase characters.

<SignatureBlock label="Signature">toUpperCase(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/toUpperCase.md -->

---

#### `toLowerCase()`

Returns the string with all lowercase characters.

<SignatureBlock label="Signature">toLowerCase(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/toLowerCase.md -->

### Case transformation

---

#### `toSlugCase()`

Normalizes the string into a URL-friendly slug. Strips accents via NFKD decomposition, removes non-alphanumeric characters, and joins words with hyphens.

<SignatureBlock label="Signature">toSlugCase(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/toSlugCase.md -->

---

#### `toDashCase()`

Converts the string to dash-case (lowercase words joined by hyphens). Tokenizes on whitespace, underscores, hyphens and camelCase boundaries. Unlike `toSlugCase()`, accents are preserved.

<SignatureBlock label="Signature">toDashCase(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/toDashCase.md -->

---

#### `toKebabCase()`

Alias for `toDashCase()`.

<SignatureBlock label="Signature">toKebabCase(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/toKebabCase.md -->

---

#### `toPascalCase()`

Converts the string to PascalCase. Each word is capitalized and joined without separators.

<SignatureBlock label="Signature">toPascalCase(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/toPascalCase.md -->

---

#### `toCamelCase()`

Converts the string to camelCase. The first word is lowercase, subsequent words are capitalized.

<SignatureBlock label="Signature">toCamelCase(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/toCamelCase.md -->

---

#### `toSnakeCase()`

Converts the string to snake_case (lowercase words joined by underscores).

<SignatureBlock label="Signature">toSnakeCase(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/toSnakeCase.md -->

### Trimming

---

#### `trim()`

Removes whitespace from both ends of the string.

<SignatureBlock label="Signature">trim(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/trim.md -->

---

#### `trimStart()`

Removes whitespace from the start of the string.

<SignatureBlock label="Signature">trimStart(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/trimStart.md -->

---

#### `trimEnd()`

Removes whitespace from the end of the string.

<SignatureBlock label="Signature">trimEnd(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/trimEnd.md -->

### Searching

---

#### `includes()`

Checks if the string contains a substring.

<SignatureBlock label="Signature">includes(search: string): boolean</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/includes.md -->

---

#### `startsWith()`

Checks if the string starts with a substring.

<SignatureBlock label="Signature">startsWith(search: string): boolean</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/startsWith.md -->

---

#### `endsWith()`

Checks if the string ends with a substring.

<SignatureBlock label="Signature">endsWith(search: string): boolean</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/endsWith.md -->

---

#### `indexOf()`

Returns the index of the first occurrence of a substring, or `-1`.

<SignatureBlock label="Signature">indexOf(search: string, start?: i64): i64</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/indexOf.md -->

---

#### `lastIndexOf()`

Returns the index of the last occurrence of a substring, or `-1`.

<SignatureBlock label="Signature">lastIndexOf(search: string, start?: i64): i64</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/lastIndexOf.md -->

---

#### `search()`

Searches for a pattern using a regular expression and returns the index of the match.

<SignatureBlock label="Signature">search(pattern: Regex): i64</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/search.md -->

### Slicing and extracting

---

#### `slice()`

Extracts a section of the string and returns it as a new string.

<SignatureBlock label="Signature">slice(start?: i64, end?: i64): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/slice.md -->

---

#### `substring()`

Returns a substring starting at a given index.

<SignatureBlock label="Signature">substring(start: i64, end?: i64): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/substring.md -->

---

#### `charAt()`

Returns the character at a specified index.

<SignatureBlock label="Signature">charAt(index: i64): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/charAt.md -->

---

#### `charCodeAt()`

Returns the Unicode code point of the character at a specified index.

<SignatureBlock label="Signature">charCodeAt(index: i64): u32</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/charCodeAt.md -->

---

#### `truncate()`

Truncates the string to a maximum number of code points.

<SignatureBlock label="Signature">truncate(maxLength: i64): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/truncate.md -->

---

#### `truncateEllipsis()`

Truncates the string to a maximum number of code points and appends `...` when the text overflows.

<SignatureBlock label="Signature">truncateEllipsis(maxLength: i64): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/truncateEllipsis.md -->

### Transforming

---

#### `replace()`

Replaces a substring or pattern with a replacement string.

<SignatureBlock label="Signature">replace(search: string | Regex, replacement: string): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/replace.md -->

---

#### `repeat()`

Returns a new string with the original repeated a given number of times.

<SignatureBlock label="Signature">repeat(count: i64): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/repeat.md -->

---

#### `padStart()`

Pads the start of the string with a filler string to reach a target length.

<SignatureBlock label="Signature">padStart(targetLength: i64, fill?: string): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/padStart.md -->

---

#### `padEnd()`

Pads the end of the string with a filler string to reach a target length.

<SignatureBlock label="Signature">padEnd(targetLength: i64, fill?: string): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/padEnd.md -->

---

#### `concat()`

Concatenates the string with one or more strings.

Keeping a `string` beyond the value it was read from, pushing it into an array, capturing it in a closure, storing it in a field, no longer copies the text. The two share one buffer and each holds a reference to it, which is sound precisely because a `string` is immutable: two readers of one buffer can never disagree.

A chain of `+` costs one allocation, not one per operator: `a + b + c + d` measures every operand first, each `string` carries its own length, so measuring reads nothing, and then fills a single buffer.

<SignatureBlock label="Signature">concat(...strings: string[]): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/concat.md -->

---

#### `split()`

Splits the string into an array of substrings using a separator.

<SignatureBlock label="Signature">split(separator: string | Regex): string[]</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/split.md -->

### HTML and encoding

---

#### `stripHtml()`

Removes all HTML tags from the string, leaving only the text content. Decodes common HTML entities (`&lt;`, `&gt;`, `&amp;`, `&quot;`, `&#39;`).

<SignatureBlock label="Signature">stripHtml(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/stripHtml.md -->

---

#### `escapeHtml()`

Escapes special characters into HTML entities (`&` `<` `>` `"` `'`), preventing XSS injection.

<SignatureBlock label="Signature">escapeHtml(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/escapeHtml.md -->

---

#### `urlEncode()`

Percent-encodes the string for use as a URL query parameter. Encodes all characters except unreserved characters (letters, digits, `-`, `_`, `.`, `~`).

<SignatureBlock label="Signature">urlEncode(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/urlEncode.md -->

---

#### `urlDecode()`

Decodes a percent-encoded string back to its original form.

<SignatureBlock label="Signature">urlDecode(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/urlDecode.md -->

### Pattern matching

---

#### `match()`

Matches the string against a regular expression and returns captured groups.

<SignatureBlock label="Signature">match(pattern: Regex): string[]?</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/match.md -->

---

#### `matchAll()`

Finds all matches of a regular expression in the string.

<SignatureBlock label="Signature">matchAll(pattern: Regex): string[][]?</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/matchAll.md -->

### Iteration

---

#### `chars()`

Iterates over the characters of the string.

<SignatureBlock label="Signature">chars(): char[]</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/chars.md -->

---

#### `graphemes()`

Iterates over the string's grapheme clusters, the characters a reader perceives, where one cluster may hold several scalar values.

<SignatureBlock label="Signature">graphemes(): string[]</SignatureBlock>

<!-- @include: ../.fragments/language/strings/methods/graphemes.md -->


## Working with Numbers

### parseInt

Parses a string and returns an optional integer. Returns empty when the input is not a valid integer representation.

<SignatureBlock label="Signature">parseInt(value: string): i64?</SignatureBlock>

<!-- @include: ../.fragments/language/strings/parseInt.md -->

### parseFloat

Parses a string and returns an optional floating-point number. Returns empty when the input is not a valid float representation.

<SignatureBlock label="Signature">parseFloat(value: string): f64?</SignatureBlock>

<!-- @include: ../.fragments/language/strings/parseFloat.md -->

### toString

Converts a value to its string representation.

<SignatureBlock label="Signature">toString(): string</SignatureBlock>

<!-- @include: ../.fragments/language/strings/toString.md -->

## Regex Integration

Strings work natively with the `Regex` type:

<!-- @include: ../.fragments/language/strings/regexp-integration.md -->

String literals are not `char`
A multi-character string literal cannot be assigned to `char`:

::: danger error
<!-- @include: ../.fragments/language/strings/error-char.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/strings/error-char-output.md -->
:::
