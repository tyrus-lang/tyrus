---
title: Arrays
description: Literales de array, indexación, métodos funcionales y comportamiento nativo de arrays.
---

# Arrays

<!-- @include: ../../.fragments/language/collections/arrays-overview.md -->

Ambas formas `T[]` y `Array<T>` son válidas. Las lecturas y escrituras indexadas realizan chequeos de límite en tiempo de ejecución. Los métodos mutantes requieren un receiver `let`. `length` informa la cantidad de elementos, mientras que `capacity` es una extensión de tyrus de solo lectura que expone el almacenamiento nativo reservado.

### Literales de array y valores repetidos

El tipo `[T; N]` declara un array con exactamente `N` elementos. Inicialízalo con una lista que contenga exactamente `N` valores o con `[valor; N]` para repetir un valor. Los literales de array sin una anotación de tamaño fijo, como `T[]`, siguen teniendo tamaño dinámico.

<!-- @include: ../../.fragments/language/collections/array-literals.md -->

### Arrays construidos por una secuencia de `push`

Un array local que empieza vacío y se llena con una secuencia directa de `push` se compila como el literal de array equivalente, lo que le permite vivir en el stack en vez del heap:

<!-- @include: ../../.fragments/language/collections/push-run.md -->

La reescritura es exacta, mismos elementos, mismo orden, mismo tamaño, y el array sigue creciendo después. Solo se aplica mientras nada haya podido observar el array en el medio: la secuencia se detiene en el primer statement que no es un `push` sobre ese array, un elemento que lee el array en construcción nunca se mueve delante de él, y un elemento con spread descalifica la secuencia porque aporta un número desconocido de elementos. Los tipos de elemento se limitan a los copiados por valor.

Nada de esto es visible en el comportamiento, solo en cuánto asigna el programa. Hacer crecer un array de otro modo asigna incluso en una build de release, ya que el crecimiento es una llamada dentro de la cual el optimizador no puede mirar.

### Chequeos de límite que un bucle ya probó

Toda lectura y escritura indexada se chequea en tiempo de ejecución, excepto donde un bucle alrededor ya probó que el chequeo no puede fallar. En la forma de conteo de abajo el cuerpo solo se alcanza cuando `i < values.length`, y nada entre esa prueba y el acceso al elemento puede cambiar ninguno de los dos lados, así que la comparación que el acceso emitiría es la condición del bucle escrita una segunda vez:

<!-- @include: ../../.fragments/language/collections/counting-loop.md -->

La prueba es deliberadamente estrecha, porque equivocarse en ella quita lo único que hay entre un índice malo y una lectura fuera del buffer. Un bucle solo califica cuando el contador se declara en su propio `init` a partir de un literal no negativo, la condición es exactamente `contador < array.length`, la actualización solo aumenta el contador por una constante, el array es un local, un parámetro o un global `const` que nada puede hacer crecer y, dentro del cuerpo, el array nunca se usa para nada más que indexación y `length`, mientras el contador nunca se escribe. Cualquier otra cosa mantiene el chequeo: un `push` en el cuerpo, un índice distinto del contador, una redeclaración de cualquiera de los dos nombres o una mención a cualquiera de ellos desde dentro de una closure.

Un chequeo que se mantiene se comporta exactamente como siempre, nombrando el índice ofensor y el tamaño del array en stderr antes de terminar con status distinto de cero.

### Chequeos de límite que una tabla fija ya probó

Un bucle no es lo único que puede resolver un índice. Cuando un binding se declara `const` a partir de un literal de array, su cantidad de elementos está escrita en el código, y un índice literal por debajo de esa cantidad está dentro del rango sin ningún bucle involucrado:

<!-- @include: ../../.fragments/language/collections/fixed-table.md -->

El chequeo de `PRIMES[2]` se elimina, y un índice más allá del final es un error de compilación en vez de uno de ejecución, `PRIMES[8]` se rechaza con `E0606`, nombrando el rango válido. Lo que fija el tamaño es el `const`, no la anotación: el binding no puede reasignarse, y un método mutante sobre él se rechaza. `readonly const` es como se escribe normalmente una tabla fija, pero un literal de array `const` simple prueba lo mismo.

El binding también tiene que ser solo leído y nada más. Pasarlo a una función, capturarlo en una closure o llamar cualquier método sobre él termina la prueba y el chequeo permanece, la misma regla estrecha que sigue la forma de bucle, y por el mismo motivo.


::: warning Los métodos mutables requieren `let`
Un array `const` es profundamente inmutable. Llamar a `push`, `sort`, `reverse` o a cualquier otro método mutable en un array `const` produce un error de compilación.
:::

::: danger error
<!-- @include: ../../.fragments/language/collections/const-mutate-error.md -->
:::

## Propiedades

| Miembro | Tipo | Comportamiento |
| --- | --- | --- |
| `length` | `usize` | Cantidad actual de elementos. |
| `capacity` | `usize` | Almacenamiento nativo reservado; es una extensión de Tyrus. |

## Métodos mutables

Estos métodos modifican el array y, por eso, requieren un binding `let` mutable. `pop()` y `shift()` devuelven `T?`, así que un array vacío produce ausencia.

| Método | Firma | Comportamiento |
| --- | --- | --- |
| `push` | `push(...items: T[]): usize` | Añade valores al final y devuelve la nueva longitud. |
| `pop` | `pop(): T?` | Elimina y devuelve el último valor, o ausencia. |
| `unshift` | `unshift(...items: T[]): usize` | Añade valores al inicio y devuelve la nueva longitud. |
| `shift` | `shift(): T?` | Elimina y devuelve el primer valor, o ausencia. |
| `splice` | `splice(start: i64, deleteCount?: i64, ...items: T[]): T[]` | Elimina, reemplaza o inserta valores y devuelve los valores eliminados. |

`sort`, `reverse`, `rotate`, `fill` y `copyWithin` también modifican el receiver. Su comportamiento se explica en [Métodos funcionales](#metodos-funcionales).

## Métodos estáticos

| Método | Firma | Comportamiento |
| --- | --- | --- |
| `Array.of` | `Array.of(...items: T[]): T[]` | Crea un array con sus argumentos. |
| `Array.from` | `Array.from(items: T[]): T[]` | Copia un array en un array de tamaño dinámico. |
| `Array.withCapacity` | `Array.withCapacity(capacity: usize): T[]` | Crea un array vacío con capacidad reservada. |
| `Array.isArray` | `Array.isArray(value: unknown): boolean` | Comprueba si un valor es un array. |

## Métodos Funcionales

| Categoría | Métodos |
| --- | --- |
| Iteración & Transformación | [`map()`](#map), [`filter()`](#filter), [`partition()`](#partition), [`reduce()`](#reduce), [`reduceRight()`](#reduceright), [`forEach()`](#foreach) |
| Ventanas y Matrices | [`windows()`](#windows), [`scan()`](#scan), [`cartesianProduct()`](#cartesianproduct), [`zip()`](#zip), [`transpose()`](#transpose) |
| Deduplicación y Agrupación | [`unique()`](#unique), [`uniqueBy()`](#uniqueby), [`groupBy()`](#groupby) |
| Toggle | [`toggle()`](#toggle), [`toggleAll()`](#toggleall) |
| Búsqueda & Test | [`find()`](#find), [`findLast()`](#findlast), [`findIndex()`](#findindex), [`findLastIndex()`](#findlastindex), [`some()`](#some), [`every()`](#every), [`includes()`](#includes), [`equals()`](#equals), [`indexOf()`](#indexof), [`lastIndexOf()`](#lastindexof) |
| Flatten & Reordenación | [`flat()`](#flat), [`flatMap()`](#flatmap), [`sort()`](#sort), [`toSorted()`](#tosorted), [`sortBy()`](#sortby), [`sortedBy()`](#sortedby), [`reverse()`](#reverse), [`rotate()`](#rotate), [`toReversed()`](#toreversed), [`random()`](#random), [`shuffle()`](#shuffle), [`sample()`](#sample) |
| Corte | [`slice()`](#slice), [`splitAt()`](#splitat) |
| Agregación | [`sum()`](#sum), [`product()`](#product), [`avg()`](#avg), [`sumBy()`](#sumby), [`avgBy()`](#avgby), [`count()`](#count), [`concat()`](#concat), [`join()`](#join) |
| Mutación | [`splice()`](#splice), [`fill()`](#fill), [`copyWithin()`](#copywithin) |
| Acceso & Iteración | [`first()`](#first), [`last()`](#last), [`at()`](#at), [`keys()`](#keys), [`values()`](#values), [`entries()`](#entries) |
| Copia Inmutable | [`with()`](#with) |

### Iteración & Transformación

Los métodos basados en predicados (`filter`, `partition`, `some`, `every`, `find`, `findLast`, `findIndex`, `findLastIndex`, `uniqueBy` y `groupBy`) aceptan una callback arrow explícita o una expresión implícita como `value > 2`; el tipo del elemento se infiere a partir del array receptor.

#### `map()`

Crea un nuevo array aplicando una transformación a cada elemento.

<SignatureBlock label="Firma">map&lt;U&gt;(callback: (element: T, index?: i64) =&gt; U): U[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-map.md -->

#### `filter()`

Selecciona elementos que satisfacen un predicado.

<SignatureBlock label="Firma">filter(predicate: (element: T) =&gt; boolean): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-filter.md -->

#### `partition()`

Separa los elementos en dos arrays estables según un predicado. El primer array contiene los elementos que coinciden; el segundo contiene los demás. El receiver no se modifica.

<SignatureBlock label="Firma">partition(predicate: (element: T) =&gt; boolean): [T[], T[]]</SignatureBlock>

<SignatureBlock label="Firma">partition(field: string): [T[], T[]]</SignatureBlock>

<SignatureBlock label="Firma">partition(field: string, ...values: string[]): [T[], T[]]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/partition.md -->

Los arrays de objetos pueden usar directamente el nombre de un campo booleano, o un campo string seguido de uno o más valores coincidentes. Los nombres de los campos se verifican contra el tipo del elemento en tiempo de compilación.

<!-- @include: ../../.fragments/language/collections/functional-partition-properties.md -->

Un campo desconocido se rechaza en tiempo de compilación.

::: danger error
<!-- @include: ../../.fragments/language/collections/functional-partition-error.md -->
:::

Para predicados más complejos, usa una arrow function explícita. Puede recibir el elemento, su índice y el array original:

<!-- @include: ../../.fragments/language/collections/partition-2.md -->

`partition()` evalúa el predicado una vez por elemento y conserva el orden original dentro de los dos arrays de salida. Al igual que `count()`, una expresión común con exactamente un identificador libre se trata como predicado implícito por elemento; una arrow function explícita sigue disponible para casos más complejos. Por seguridad de memoria, `T` debe ser `Copy`, `string` o un objeto cuyos campos son `Copy` o `string`.

#### `reduce()`

Acumula elementos en un único valor, de izquierda a derecha.

<SignatureBlock label="Firma">reduce&lt;U&gt;(callback: (accumulator: U, element: T) =&gt; U, initial: U): U</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-reduce.md -->

#### `reduceRight()`

Acumula elementos en un único valor, de derecha a izquierda.

<SignatureBlock label="Firma">reduceRight&lt;U&gt;(callback: (accumulator: U, element: T) =&gt; U, initial: U): U</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-reduceRight.md -->

#### `forEach()`

Ejecuta un callback para cada elemento. El valor de retorno se descarta.

<SignatureBlock label="Firma">forEach(callback: (element: T) =&gt; void): void</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-forEach.md -->

### Búsqueda & Test

#### `find()`

Devuelve el primer elemento que coincide con un predicado, o ausencia si ninguno coincide.

<SignatureBlock label="Firma">find(predicate: (element: T) =&gt; boolean): T?</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-find.md -->

#### `findLast()`

Devuelve el último elemento que coincide con un predicado, o ausencia si ninguno coincide.

<SignatureBlock label="Firma">findLast(predicate: (element: T) =&gt; boolean): T?</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-findLast.md -->

#### `findIndex()`

Devuelve el índice del primer elemento que coincide, o `-1`.

<SignatureBlock label="Firma">findIndex(predicate: (element: T) =&gt; boolean): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-findIndex.md -->

#### `findLastIndex()`

Devuelve el índice del último elemento que coincide, o `-1`.

<SignatureBlock label="Firma">findLastIndex(predicate: (element: T) =&gt; boolean): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-findLastIndex.md -->

#### `some()`

Comprueba si al menos un elemento satisface una condición.

<SignatureBlock label="Firma">some(predicate: (element: T) =&gt; boolean): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-some.md -->

#### `every()`

Comprueba si todos los elementos satisfacen una condición.

<SignatureBlock label="Firma">every(predicate: (element: T) =&gt; boolean): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-every.md -->

#### `includes()`

Comprueba si un valor específico existe en el array.

<SignatureBlock label="Firma">includes(value: T): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-includes.md -->

#### `equals()`

Compara dos arrays elemento a elemento: la misma longitud, los mismos elementos, en el mismo orden. `===` y `!==` no comparan arrays; el compilador informa `E0369` y remite aquí.

<SignatureBlock label="Firma">equals(other: T[]): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-equals.md -->

#### `indexOf()`

Devuelve el primer índice de un valor, o `-1` si no se encuentra.

<SignatureBlock label="Firma">indexOf(value: T, fromIndex?: i64): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-indexOf.md -->

#### `lastIndexOf()`

Devuelve el último índice de un valor, o `-1` si no se encuentra.

<SignatureBlock label="Firma">lastIndexOf(value: T, fromIndex?: i64): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-lastIndexOf.md -->

### Ventanas, Acumulación y Matrices

#### `windows()`

`windows(size)` devuelve copias superpuestas de `size` elementos consecutivos. El receiver no cambia; `windows(0)` se rechaza y un tamaño mayor que el array devuelve un array vacío.

<!-- @include: ../../.fragments/language/collections/windows.md -->

#### `scan()`

`scan(initial, callback)` devuelve el acumulador inicial seguido por cada valor acumulado. La callback recibe el acumulador y el elemento actual, y el receiver no cambia.

<!-- @include: ../../.fragments/language/collections/scan.md -->

#### `cartesianProduct()`

`cartesianProduct(other)` devuelve pares en orden lexicográfico del receiver y del argumento. Cualquier entrada vacía produce un resultado vacío, y el receiver no cambia.

<!-- @include: ../../.fragments/language/collections/cartesianproduct.md -->

#### `zip()`

`zip(other)` intercala el receiver y el argumento en un array de pares `(T, U)[]`, donde `T` es el tipo de elemento del receiver y `U` es el tipo de elemento del argumento. La longitud del resultado es igual a la de ambas entradas, que deben coincidir; los arrays de origen no cambian.

<!-- @include: ../../.fragments/language/collections/zip.md -->

Una diferencia de longitud lanza un `ZipError` recuperable (code `ERR_ARRAY_ZIP_MISMATCH`) en lugar de abortar, por lo que la llamada debe cubrirse con un `catch` o con una cláusula `throws ZipError` en la función que la contiene:

<!-- @include: ../../.fragments/language/collections/zip-2.md -->

#### `transpose()`

`transpose()` convierte una matriz rectangular en columnas. Las matrices vacías y las matrices con filas de ancho cero devuelven arrays vacíos; las filas irregulares fallan antes de producir un resultado parcial.

<!-- @include: ../../.fragments/language/collections/transpose.md -->

#### `unique()`

`unique()` devuelve la primera aparición de cada valor distinto en el orden de entrada. No muta el receiver y usa la política nativa de igualdad para strings y números.

<!-- @include: ../../.fragments/language/collections/unique.md -->

#### `uniqueBy()`

`uniqueBy(selector)` conserva el primer elemento para cada clave hashable distinta. El selector puede ser el nombre de un campo para elementos objeto, una callback explícita o una expresión implícita, cuyo tipo de elemento se infiere automáticamente.

<!-- @include: ../../.fragments/language/collections/uniqueby.md -->

#### `groupBy()`

`groupBy(selector)` crea un mapa desde cada clave hashable hacia un array de elementos en su orden original. El selector puede ser el nombre de un campo para elementos objeto, una callback explícita o una expresión implícita, cuyo tipo de elemento se infiere automáticamente. La primera implementación exige elementos Copy para que cada grupo sea propietario de sus valores de forma segura.

<!-- @include: ../../.fragments/language/collections/groupby.md -->

### Flatten & Reordenación

#### `flat()`

Aplana arrays anidados en un nivel.

<SignatureBlock label="Firma">flat(): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-flat.md -->

#### `flatMap()`

Mapea cada elemento a un array y aplana el resultado en un nivel.

<SignatureBlock label="Firma">flatMap&lt;U&gt;(callback: (element: T) =&gt; U[]): U[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-flatMap.md -->

#### `sort()`

Ordena el array in place usando una función de comparación.

<SignatureBlock label="Firma">sort(compare?: (left: T, right: T) =&gt; i32): this</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-sort.md -->

#### `toSorted()`

Devuelve una nueva copia ordenada sin mutar el original.

<SignatureBlock label="Firma">toSorted(compare?: (left: T, right: T) =&gt; i32): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-toSorted.md -->

#### `sortBy()`

Ordena un array in place por un campo resuelto estáticamente. La clave debe ser un literal string con el nombre de un campo numérico, booleano, de carácter o string. El segundo argumento es opcional: el orden predeterminado es ascendente y `"desc"` selecciona el orden descendente.

<SignatureBlock label="Firma">sortBy(key: keyof T, order?: "asc" | "desc"): this</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/sortby.md -->

El campo se resuelve en compile-time, por lo que el bucle de merge sort estable generado lo lee directamente sin reflexión en runtime. `sortBy()` requiere un receiver mutable en `let` y funciona en `O(n log n)` con `O(n)` de espacio auxiliar.

#### `sortedBy()`

Devuelve una copia ordenada por un campo resuelto estáticamente y deja el receiver sin cambios. El segundo argumento es opcional: el orden predeterminado es ascendente y `"desc"` selecciona el orden descendente. La primera versión requiere que los campos del elemento sean `Copy`, evitando una copia superficial insegura de valores poseídos. Usa merge sort estable `O(n log n)` con `O(n)` de espacio auxiliar.

<SignatureBlock label="Firma">sortedBy(key: keyof T, order?: "asc" | "desc"): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/sortedby.md -->

#### `reverse()`

Invierte el orden de los elementos in place.

<SignatureBlock label="Firma">reverse(): this</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-reverse.md -->

#### `rotate()`

Rota el array in place. Los valores positivos rotan hacia la izquierda; los negativos rotan hacia la derecha. El valor se normaliza por la longitud del array, y los arrays vacíos o de un solo elemento no cambian.

<SignatureBlock label="Firma">rotate(amount: i64): this</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/rotate.md -->

`rotate()` requiere un receiver mutable en `let` y se ejecuta en `O(n)` con espacio auxiliar `O(1)`, sin asignar un segundo array.

#### `toReversed()`

Devuelve una nueva copia con orden invertido sin mutar el original.

<SignatureBlock label="Firma">toReversed(): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-toReversed.md -->

#### `random()`

Devuelve un `T` seleccionado uniformemente. Un array vacío aborta en runtime.

<SignatureBlock label="Firma">random(): T</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/random.md -->

::: danger error
<!-- @include: ../../.fragments/language/collections/random-2.md -->

<!-- @include: ../../.fragments/language/collections/random-3.md -->
:::

#### `shuffle()`

Devuelve un nuevo array mezclado uniformemente sin mutar el receptor. Los arrays con menos de dos elementos abortan en runtime.

<SignatureBlock label="Firma">shuffle(): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/shuffle.md -->

::: danger error
<!-- @include: ../../.fragments/language/collections/shuffle-2.md -->

<!-- @include: ../../.fragments/language/collections/shuffle-3.md -->
:::

#### `sample()`

Devuelve un nuevo array con `count` elementos seleccionados uniformemente. `count` debe estar entre `1` y la longitud del receptor, inclusive. Sin opciones, las posiciones de origen no se repiten. `{ repeat: true }` permite repeticiones de posiciones, pero no las garantiza.

<SignatureBlock label="Firma">sample(count: usize, options?: { repeat?: boolean }): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/sample.md -->

::: danger error
<!-- @include: ../../.fragments/language/collections/sample-2.md -->

<!-- @include: ../../.fragments/language/collections/sample-3.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/collections/sample-4.md -->

<!-- @include: ../../.fragments/language/collections/sample-5.md -->
:::

### Corte

#### `slice()`

Extrae una copia de un subarray contiguo.

<SignatureBlock label="Firma">slice(start?: i64, end?: i64): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-slice.md -->

Indexar con un `Range` es azúcar para `.slice()`: `arr[a..b]` es `arr.slice(a, b)`, y toda forma abierta también funciona, `arr[a..]`, `arr[..b]`, `arr[..=b]` y el rango completo `arr[..]`. Igual que `.slice()`, esto siempre copia a un array nuevo; los arrays de tyrus todavía no tienen representación de slice prestado sin copia.

<!-- @include: ../../.fragments/language/collections/range-slicing.md -->

#### `splitAt()`

Divide el array en dos arrays en un índice dado.

<SignatureBlock label="Firma">splitAt(index: i64): [T[], T[]]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-splitAt.md -->

`splitAt` siempre devuelve dos arrays comunes de tamaño dinámico (`[T[], T[]]`), incluso cuando se llama sobre un array de tamaño fijo. `index` puede ser igual a la longitud del array, produciendo una segunda mitad vacía, pero un índice mayor es un error de compilación cuando la longitud del array se conoce estáticamente, y un abort en tiempo de ejecución en caso contrario.

### Agregación

#### `count()`

Devuelve la longitud, o cuenta ocurrencias de un valor o predicado.

<SignatureBlock label="Firma">count(value?: T | (element: T) =&gt; boolean): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-count.md -->

`count()` devuelve la longitud; `count(valor)` cuenta elementos iguales a `valor`; `count(condición)`, una expresión común que referencia exactamente un identificador todavía no declarado en el ámbito actual, se convierte en una condición implícita por elemento, azúcar para `count((implicitParam): boolean => condición)` sin necesidad de escribir ninguna arrow function. Un identificador que ya resuelve en el ámbito no es libre, así que la expresión se comprueba como una llamada común `count(valor)`, comparar su resultado (constante) `boolean` contra el tipo del elemento es un error de compilación que cita el identificador que ensombreció el parámetro implícito. Una expresión que referencia más de un identificador no declarado también es un error de compilación, ya que el azúcar solo introduce un parámetro implícito.

#### `sum()`

Devuelve la suma de todos los elementos numéricos usando el tipo de los elementos del array. Un array numérico vacío devuelve el valor cero de ese tipo.

<SignatureBlock label="Firma">sum(): T</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/sum.md -->

`sum()` se rechaza para tipos de elemento no numéricos y se compila como un bucle nativo directo, sin callback ni despacho al runtime por elemento.

#### `product()`

Devuelve el producto de todos los elementos numéricos usando el tipo de los elementos del array. Un array numérico vacío devuelve el valor uno de ese tipo.

<SignatureBlock label="Firma">product(): T</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/product.md -->

`product()` sigue las mismas reglas de tipo numérico y bucle nativo que `sum()`.

#### `avg()`

Devuelve el promedio aritmético de los elementos numéricos. Los arrays de enteros, `f32` y `f64` devuelven `f64?`; los arrays de `f128` devuelven `f128?`. Un array vacío produce ausencia.

<SignatureBlock label="Firma">avg(): f64? | f128?</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/avg.md -->

El resultado opcional debe tratarse con `??`, narrowing u otra operación válida de opcionales antes del uso numérico directo.

#### `sumBy()`

Retorna la suma de un campo numérico en todos los elementos. La clave debe ser un literal de string que nombre un campo numérico.

<SignatureBlock label="Firma">sumBy(key: string): T[K]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/sumby.md -->

#### `avgBy()`

Retorna el promedio aritmético de un campo numérico en todos los elementos. La clave debe ser un literal de string que nombre un campo numérico. Siempre retorna `f64?` (o `f128?` para campos `f128`).

<SignatureBlock label="Firma">avgBy(key: string): f64? | f128?</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/avgby.md -->

#### `concat()`

Fusiona dos arrays en uno nuevo.

<SignatureBlock label="Firma">concat(...items: (T | T[])[]): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-concat.md -->

#### `join()`

Une todos los elementos en una sola cadena usando un separador.

<SignatureBlock label="Firma">join(separator: string): string</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-join.md -->

### Mutación

#### `splice()`

Elimina, reemplaza o inserta elementos in place.

<SignatureBlock label="Firma">splice(start: i64, deleteCount?: i64, ...items: T[]): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-splice.md -->

`splice(start, deleteCount?, ...items)` modifica el receptor y devuelve un nuevo array con los elementos eliminados. Los índices iniciales negativos cuentan desde el final, la eliminación se limita al intervalo disponible y los valores insertados deben coincidir con el tipo de los elementos del array.

#### `fill()`

Reemplaza todos los elementos con un valor fijo.

<SignatureBlock label="Firma">fill(value: T): this</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-fill.md -->

#### `copyWithin()`

Copia una sección contigua del array a otra posición en el mismo array.

<SignatureBlock label="Firma">copyWithin(target: i64, start: i64, end?: i64): this</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-copyWithin.md -->

### Acceso & Iteración

#### `first()`

Devuelve el primer elemento como `T?`. Un array vacío produce ausencia. La anotación de tipo es opcional: cuando `values` es `i32[]`, tyrus infiere `i32?` automáticamente. `Optional<i32>` es la forma explícita equivalente.

<SignatureBlock label="Firma">first(): T?</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/first.md -->

`T?` es intencional: `first()` devuelve un valor, no una colección, y sigue siendo `O(1)` sin asignar un array vacío como resultado. El uso directo como `i32` se rechaza hasta que un `if`, `??`, ternario, optional chaining u otra operación válida de narrowing trate la ausencia.

#### `last()`

Devuelve el último elemento como `T?`. Un array vacío produce ausencia, y tyrus infiere automáticamente el tipo opcional del elemento cuando se omite la anotación.

<SignatureBlock label="Firma">last(): T?</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/last.md -->

`last()` sigue las mismas reglas de ausencia y narrowing que `first()`, leyendo el elemento final en `O(1)` sin asignar un array de resultado.

#### `at()`

Devuelve el elemento en un índice, soportando indexación negativa desde el final.

<SignatureBlock label="Firma">at(index: i64): T</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-at.md -->

#### `keys()`

Devuelve un iterador sobre los índices del array.

<SignatureBlock label="Firma">keys(): Iterable&lt;i64&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-keys.md -->

#### `values()`

Devuelve un iterador sobre los valores del array.

<SignatureBlock label="Firma">values(): Iterable&lt;T&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-values.md -->

#### `entries()`

Devuelve un iterador sobre pares índice-valor.

<SignatureBlock label="Firma">entries(): Iterable&lt;[i64, T]&gt;</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-entries.md -->

### Copia Inmutable

#### `with()`

Devuelve un nuevo array con un único elemento reemplazado en un índice dado.

<SignatureBlock label="Firma">with(index: i64, value: T): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/functional-with.md -->

### Toggle

#### `toggle()`

Agrega un elemento al array si no está presente y lo elimina si está presente, devolviendo un array nuevo y sin mutar nunca el receptor.

<SignatureBlock label="Firma">toggle(item: T): T[]</SignatureBlock>

<SignatureBlock label="Firma">toggle(item: T, key: string): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/toggle.md -->

Cuando se proporciona un nombre de campo `key`, la comparación se hace por ese campo en lugar de por el elemento completo. El campo debe existir en el tipo del elemento.

<!-- @include: ../../.fragments/language/collections/toggle-2.md -->

#### `toggleAll()`

Aplica el mismo comportamiento de agregar o eliminar a cada elemento del array argumento, preservando el orden. No muta nada.

<SignatureBlock label="Firma">toggleAll(items: T[]): T[]</SignatureBlock>

<!-- @include: ../../.fragments/language/collections/toggleall.md -->

### Encadenamiento

Los métodos funcionales se encadenan en pipelines de datos legibles:

<!-- @include: ../../.fragments/language/collections/functional-chaining.md -->
