---
title: Cadenas de texto
description: Literales de cadena, template literals, interpolación, texto multilínea y tipos de cadena literales en tyrus.
---

# Cadenas de texto

`string` es el tipo nativo de texto UTF-8 en tyrus. La sintaxis fuente es concisa, mientras que la implementación de runtime es nativa.

## Cadenas con comillas

<!-- @include: ../../.fragments/language/strings/quoted-strings.md -->

El formateador normaliza las comillas según `quote_style`, salvo cuando se configura como `"off"`, que preserva las comillas y escapes originales de cada literal de cadena.

## Cadenas multilínea

Una cadena delimitada por `"""` puede ocupar varias líneas manteniendo la alineación con el código circundante. El compilador la desindenta en tiempo de compilación: de cada línea, quita tantos espacios a la izquierda como los que tenga el `"""` de cierre, y descarta una primera línea dejada en blanco justo después del `"""` de apertura. El resultado es un literal `string` común, así que no hay costo en runtime por escribir una consulta o una plantilla alineada en lugar de una pegada al margen izquierdo.

<!-- @include: ../../.fragments/language/strings/multiline-strings.md -->

El margen de desindentación solo cuenta espacios; un `"""` cerrado en una línea indentada con tabulaciones mide un margen de cero, así que usa espacios para la indentación que quieras eliminar.

## Template literals

Usa template literals para interpolación y texto multilínea.

<!-- @include: ../../.fragments/language/strings/template-literals.md -->

Una interpolación puede llevar un especificador de formato después de `:`, `.Nf` fija un float en exactamente `N` decimales, y `0Nd` rellena un entero con ceros a la izquierda hasta un ancho total de `N` caracteres. `.Nf` exige un valor float y `0Nd` exige un valor entero; un desajuste es un error de compilación (`E0717`) que nombra el tipo requerido.

Un valor de tipo unión también puede interpolarse. Qué variante contiene es un hecho de tiempo de ejecución, así que la renderización se elige en tiempo de ejecución y coincide con lo que `print` imprimiría para esa variante.

## Tipos de cadena literales

Los literales de cadena pueden participar en uniones literales y uniones etiquetadas.

Una unión de literales es un valor común: compárala con cualquiera de sus variantes (`status === "active"`), ramifica con un `match` que no necesita `_` cuando cubre toda variante, construye una con un ternario, júntalas en un array y llama en ella cualquier cosa que una `string` acepte, toda variante de una unión de literales de string es una `string`. La comparación es un chequeo de un byte en vez de comparación de texto, porque una unión así se representa por qué variante contiene y nada más.

<!-- @include: ../../.fragments/language/strings/literal-string-types.md -->

::: warning Los valores literales no son miembros de enum
Usa literales de cadena para dominios textuales cerrados pequeños. Usa `enum` cuando el dominio necesita miembros nombrados y semántica nativa de enum.
:::

## Grupos de grafemas

`chars()` devuelve valores escalares Unicode; `graphemes()` devuelve lo que un lector llama caracteres. Los dos difieren siempre que un carácter visible se escribe con varios valores escalares, un emoji armado con secuencia de zero-width joiner, o una letra seguida de acento combinante.

<!-- @include: ../../.fragments/language/strings/graphemes.md -->

Indexación, corte y `length` cuentan valores escalares, no grupos: esa es la unidad en la que se mide toda posición de la API de cadenas, y es lo que hace que `text.charAt(text.length - 1)` sea el último. `graphemes()` responde a la pregunta de visualización, cuántos caracteres ve un lector, y dónde se detendría un cursor, y devuelve `string[]`, porque un grupo puede contener más de un valor escalar.

## Propiedades de cadena

### length

<!-- @include: ../../.fragments/language/strings/length.md -->

`length` se lee del valor, no se cuenta: una `string` lleva su recuento de caracteres junto al texto, así que preguntarlo cuesta lo mismo con tres caracteres que con tres millones. U+0000 es un carácter corriente en una `string`, el texto lo delimita la longitud registrada, no un terminador, así que `"a\u{0}b".length` es `3` y sobrevive entero a la comparación, la interpolación y la impresión.

`length` cuenta caracteres, valores escalares Unicode, nunca los bytes UTF-8 subyacentes, por lo que coincide con `chars()` en cualquier texto. Toda posición de la API de cadenas usa esa misma unidad: `charAt()`, `charCodeAt()`, `slice()`, `substring()`, `indexOf()`, `lastIndexOf()`, `padStart()` y `padEnd()` cuentan caracteres, que es lo que hace que `text.charAt(text.length - 1)` sea el último carácter de cualquier cadena, acentuada o no.

## Métodos de cadena

| Categoría | Métodos |
| --- | --- |
| Conversión de mayúsculas/minúsculas | [`toUpperCase()`](#touppercase), [`toLowerCase()`](#tolowercase) |
| Transformación de caso | [`toSlugCase()`](#toslugcase), [`toDashCase()`](#todashcase), [`toKebabCase()`](#tokebabcase), [`toPascalCase()`](#topascalcase), [`toCamelCase()`](#tocamelcase), [`toSnakeCase()`](#tosnakecase) |
| Eliminación de espacios | [`trim()`](#trim), [`trimStart()`](#trimstart), [`trimEnd()`](#trimend) |
| Búsqueda | [`includes()`](#includes), [`startsWith()`](#startswith), [`endsWith()`](#endswith), [`indexOf()`](#indexof), [`lastIndexOf()`](#lastindexof), [`search()`](#search) |
| Rebanado y extracción | [`slice()`](#slice), [`substring()`](#substring), [`charAt()`](#charat), [`charCodeAt()`](#charcodeat), [`truncate()`](#truncate), [`truncateEllipsis()`](#truncateellipsis) |
| Transformación | [`replace()`](#replace), [`repeat()`](#repeat), [`padStart()`](#padstart), [`padEnd()`](#padend), [`concat()`](#concat), [`split()`](#split) |
| HTML y codificación | [`stripHtml()`](#striphtml), [`escapeHtml()`](#escapehtml), [`urlEncode()`](#urlencode), [`urlDecode()`](#urldecode) |
| Coincidencia de patrones | [`match()`](#match), [`matchAll()`](#matchall) |
| Iteración | [`chars()`](#chars), [`graphemes()`](#graphemes) |

### Conversión de mayúsculas/minúsculas

---

#### `toUpperCase()`

Devuelve la cadena con todos los caracteres en mayúscula.

<SignatureBlock label="Firma">toUpperCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toUpperCase.md -->

---

#### `toLowerCase()`

Devuelve la cadena con todos los caracteres en minúscula.

<SignatureBlock label="Firma">toLowerCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toLowerCase.md -->

### Transformación de caso

---

#### `toSlugCase()`

Normaliza la cadena en un slug amigable para URLs. Elimina acentos mediante descomposición NFKD, elimina caracteres no alfanuméricos y une palabras con guiones.

<SignatureBlock label="Firma">toSlugCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toSlugCase.md -->

---

#### `toDashCase()`

Convierte la cadena a dash-case (palabras en minúscula unidas por guiones). Tokeniza por espacios, guiones bajos, guiones y límites de camelCase. A diferencia de `toSlugCase()`, los acentos se preservan.

<SignatureBlock label="Firma">toDashCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toDashCase.md -->

---

#### `toKebabCase()`

Alias de `toDashCase()`.

<SignatureBlock label="Firma">toKebabCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toKebabCase.md -->

---

#### `toPascalCase()`

Convierte la cadena a PascalCase. Cada palabra se capitaliza y se une sin separadores.

<SignatureBlock label="Firma">toPascalCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toPascalCase.md -->

---

#### `toCamelCase()`

Convierte la cadena a camelCase. La primera palabra está en minúscula, las siguientes se capitalizan.

<SignatureBlock label="Firma">toCamelCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toCamelCase.md -->

---

#### `toSnakeCase()`

Convierte la cadena a snake_case (palabras en minúscula unidas por guiones bajos).

<SignatureBlock label="Firma">toSnakeCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toSnakeCase.md -->

### Eliminación de espacios

---

#### `trim()`

Elimina espacios en blanco de ambos extremos de la cadena.

<SignatureBlock label="Firma">trim(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/trim.md -->

---

#### `trimStart()`

Elimina espacios en blanco del inicio de la cadena.

<SignatureBlock label="Firma">trimStart(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/trimStart.md -->

---

#### `trimEnd()`

Elimina espacios en blanco del final de la cadena.

<SignatureBlock label="Firma">trimEnd(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/trimEnd.md -->

### Búsqueda

---

#### `includes()`

Comprueba si la cadena contiene una subcadena.

<SignatureBlock label="Firma">includes(search: string): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/includes.md -->

---

#### `startsWith()`

Comprueba si la cadena comienza con una subcadena.

<SignatureBlock label="Firma">startsWith(search: string): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/startsWith.md -->

---

#### `endsWith()`

Comprueba si la cadena termina con una subcadena.

<SignatureBlock label="Firma">endsWith(search: string): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/endsWith.md -->

---

#### `indexOf()`

Devuelve el índice de la primera aparición de una subcadena, o `-1`.

<SignatureBlock label="Firma">indexOf(search: string, start?: i64): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/indexOf.md -->

---

#### `lastIndexOf()`

Devuelve el índice de la última aparición de una subcadena, o `-1`.

<SignatureBlock label="Firma">lastIndexOf(search: string, start?: i64): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/lastIndexOf.md -->

---

#### `search()`

Busca un patrón usando una expresión regular y devuelve el índice de la coincidencia.

<SignatureBlock label="Firma">search(pattern: Regex): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/search.md -->

### Rebanado y extracción

---

#### `slice()`

Extrae una sección de la cadena y la devuelve como una nueva cadena.

<SignatureBlock label="Firma">slice(start?: i64, end?: i64): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/slice.md -->

---

#### `substring()`

Devuelve una subcadena a partir de un índice dado.

<SignatureBlock label="Firma">substring(start: i64, end?: i64): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/substring.md -->

---

#### `charAt()`

Devuelve el carácter en un índice especificado.

<SignatureBlock label="Firma">charAt(index: i64): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/charAt.md -->

---

#### `charCodeAt()`

Devuelve el código Unicode del carácter en un índice especificado.

<SignatureBlock label="Firma">charCodeAt(index: i64): u32</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/charCodeAt.md -->

---

#### `truncate()`

Trunca la cadena a un número máximo de code points.

<SignatureBlock label="Firma">truncate(maxLength: i64): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/truncate.md -->

---

#### `truncateEllipsis()`

Trunca la cadena a un número máximo de code points y agrega `...` cuando el texto excede el límite.

<SignatureBlock label="Firma">truncateEllipsis(maxLength: i64): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/truncateEllipsis.md -->

### Transformación

---

#### `replace()`

Reemplaza una subcadena o patrón con una cadena de reemplazo.

<SignatureBlock label="Firma">replace(search: string | Regex, replacement: string): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/replace.md -->

---

#### `repeat()`

Devuelve una nueva cadena con la original repetida un número dado de veces.

<SignatureBlock label="Firma">repeat(count: i64): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/repeat.md -->

---

#### `padStart()`

Rellena el inicio de la cadena con un carácter de relleno hasta alcanzar una longitud objetivo.

<SignatureBlock label="Firma">padStart(targetLength: i64, fill?: string): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/padStart.md -->

---

#### `padEnd()`

Rellena el final de la cadena con un carácter de relleno hasta alcanzar una longitud objetivo.

<SignatureBlock label="Firma">padEnd(targetLength: i64, fill?: string): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/padEnd.md -->

---

#### `concat()`

Concatena la cadena con una o más cadenas.

Conservar una `string` más allá del valor del que se leyó, meterla en un arreglo, capturarla en un cierre, guardarla en un campo, ya no copia el texto. Ambas comparten un búfer y cada una mantiene una referencia a él, lo cual es seguro precisamente porque una `string` es inmutable: dos lectores de un búfer no pueden discrepar.

Una cadena de `+` cuesta una asignación, no una por operador: `a + b + c + d` mide todos los operandos primero, cada `string` lleva su propia longitud, así que medir no lee nada, y solo entonces llena un único búfer.

<SignatureBlock label="Firma">concat(...strings: string[]): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/concat.md -->

---

#### `split()`

Divide la cadena en un array de subcadenas usando un separador.

<SignatureBlock label="Firma">split(separator: string | Regex): string[]</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/split.md -->

### HTML y codificación

---

#### `stripHtml()`

Elimina todas las etiquetas HTML de la cadena, dejando solo el contenido textual. Decodifica entidades HTML comunes (`&lt;`, `&gt;`, `&amp;`, `&quot;`, `&#39;`).

<SignatureBlock label="Firma">stripHtml(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/stripHtml.md -->

---

#### `escapeHtml()`

Escapa caracteres especiales a entidades HTML (`&` `<` `>` `"` `'`), previniendo inyección XSS.

<SignatureBlock label="Firma">escapeHtml(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/escapeHtml.md -->

---

#### `urlEncode()`

Codifica la cadena con percent-encoding para uso como parámetro de query URL. Codifica todos los caracteres excepto los no reservados (letras, dígitos, `-`, `_`, `.`, `~`).

<SignatureBlock label="Firma">urlEncode(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/urlEncode.md -->

---

#### `urlDecode()`

Decodifica una cadena codificada con percent-encoding de vuelta a su forma original.

<SignatureBlock label="Firma">urlDecode(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/urlDecode.md -->

### Coincidencia de patrones

---

#### `match()`

Compara la cadena con una expresión regular y devuelve grupos capturados.

<SignatureBlock label="Firma">match(pattern: Regex): string[]?</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/match.md -->

---

#### `matchAll()`

Encuentra todas las coincidencias de una expresión regular en la cadena.

<SignatureBlock label="Firma">matchAll(pattern: Regex): string[][]?</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/matchAll.md -->

### Iteración

---

#### `chars()`

Itera sobre los caracteres de la cadena.

<SignatureBlock label="Firma">chars(): char[]</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/chars.md -->

---

#### `graphemes()`

Itera sobre los grupos de grafemas de la cadena, los caracteres que un lector percibe, donde un grupo puede contener varios valores escalares.

<SignatureBlock label="Firma">graphemes(): string[]</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/graphemes.md -->


## Trabajando con números

### parseInt

Analiza una cadena y devuelve un entero opcional. Devuelve vacío cuando la entrada no es una representación válida de entero.

<SignatureBlock label="Firma">parseInt(value: string): i64?</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/parseInt.md -->

### parseFloat

Analiza una cadena y devuelve un número de punto flotante opcional. Devuelve vacío cuando la entrada no es una representación válida de flotante.

<SignatureBlock label="Firma">parseFloat(value: string): f64?</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/parseFloat.md -->

### toString

Convierte un valor a su representación de cadena.

<SignatureBlock label="Firma">toString(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/toString.md -->

## Integración con Regex

Las cadenas funcionan de forma nativa con el tipo `Regex`:

<!-- @include: ../../.fragments/language/strings/regexp-integration.md -->

Los literales de cadena no son `char`
Un literal de cadena de múltiples caracteres no puede asignarse a `char`:

::: danger error
<!-- @include: ../../.fragments/language/strings/error-char.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/strings/error-char-output.md -->
:::
