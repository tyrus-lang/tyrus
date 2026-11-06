---
title: Arrays
description: Array literals, indexing, functional methods, and native array behavior.
---

# Arrays

<!-- @include: ../.fragments/language/collections/arrays-overview.md -->

Both `T[]` and `Array<T>` are accepted. Indexed reads and writes perform runtime bounds checks. Mutating methods require a `let` receiver. `length` reports the element count, while `capacity` is a read-only tyrus extension exposing reserved native storage.

### Array literals and repeated values

The type `[T; N]` declares an array with exactly `N` elements. Initialize it with a list containing exactly `N` values or with `[value; N]` to repeat one value. Array literals without a fixed-size annotation, such as `T[]`, remain dynamically sized.

<!-- @include: ../.fragments/language/collections/array-literals.md -->

### Arrays built by a run of `push`

A local array that starts empty and is filled by a straight run of `push` is compiled as the array literal it is equivalent to, which lets it live on the stack instead of the heap:

<!-- @include: ../.fragments/language/collections/push-run.md -->

The rewrite is exact, same elements, same order, same length, and the array still grows afterwards. It applies only while nothing can have observed the array in between: the run stops at the first statement that is not a `push` onto that array, an element that reads the array being built is never moved ahead of it, and a spread element disqualifies the run because it contributes an unknown number of elements. Element types are limited to those copied by value.

Nothing about this is visible in behaviour, only in how much the program allocates. Growing an array otherwise allocates even in a release build, since the growth is a call the optimizer cannot look inside.

### Bounds checks a loop already proved

Every indexed read and write is checked at run time, except where a surrounding loop already proved the check cannot fail. In the counting shape below the body is only reached when `i < values.length`, and nothing between that test and the element access can change either side of it, so the comparison the access would emit is the loop condition written a second time:

<!-- @include: ../.fragments/language/collections/counting-loop.md -->

The proof is deliberately narrow, because being wrong about it removes the only thing standing between a bad index and a read outside the buffer. A loop qualifies only when the counter is declared by its own `init` from a non-negative literal, the condition is exactly `counter < array.length`, the update only increases the counter by a constant, the array is a local, a parameter, or a `const` global nothing can grow, and, inside the body, the array is never used for anything but indexing and `length` while the counter is never written. Anything else keeps its check: a `push` in the body, an index other than the counter, a rebinding of either name, or a mention of either from inside a closure.

A check that is kept behaves exactly as it always did, naming the offending index and the array's length on stderr before exiting non-zero.

### Bounds checks a fixed table already proved

A loop is not the only thing that can settle an index. When a binding is declared `const` from an array literal, its element count is written in the source, and a literal index below that count is in range with no loop involved:

<!-- @include: ../.fragments/language/collections/fixed-table.md -->

The check for `PRIMES[2]` is dropped, and an index past the end is a compile error rather than a runtime one, `PRIMES[8]` is rejected with `E0606`, naming the valid range. What fixes the length is `const`, not the annotation: the binding cannot be reassigned, and a mutating method on it is rejected. `readonly const` is how a fixed table is normally written, but a plain `const` array literal proves the same thing.

The binding also has to be read and nothing else. Passing it to a function, capturing it in a closure, or calling any method on it ends the proof and the check stays, the same narrow rule the loop shape follows, and for the same reason.


::: warning Mutating methods require `let`
A `const` array is deeply immutable. Calling `push`, `sort`, `reverse`, or other mutating methods on a `const` array is a compile error.
:::

::: danger error
<!-- @include: ../.fragments/language/collections/const-mutate-error.md -->
:::

## Properties

| Member | Type | Behavior |
| --- | --- | --- |
| `length` | `usize` | Current number of elements. |
| `capacity` | `usize` | Reserved native storage; this is a Tyrus extension. |

## Mutable methods

These methods change the array and therefore require a mutable `let` binding. `pop()` and `shift()` return `T?`, so an empty array produces absence.

| Method | Signature | Behavior |
| --- | --- | --- |
| `push` | `push(...items: T[]): usize` | Appends values and returns the new length. |
| `pop` | `pop(): T?` | Removes and returns the last value, or absence. |
| `unshift` | `unshift(...items: T[]): usize` | Prepends values and returns the new length. |
| `shift` | `shift(): T?` | Removes and returns the first value, or absence. |
| `splice` | `splice(start: i64, deleteCount?: i64, ...items: T[]): T[]` | Removes, replaces, or inserts values and returns the removed values. |

`sort`, `reverse`, `rotate`, `fill`, and `copyWithin` also mutate the receiver. Their behavior is covered in [Functional Methods](#functional-methods).

## Static methods

| Method | Signature | Behavior |
| --- | --- | --- |
| `Array.of` | `Array.of(...items: T[]): T[]` | Creates an array from its arguments. |
| `Array.from` | `Array.from(items: T[]): T[]` | Copies an array into a dynamically sized array. |
| `Array.withCapacity` | `Array.withCapacity(capacity: usize): T[]` | Creates an empty array with reserved capacity. |
| `Array.isArray` | `Array.isArray(value: unknown): boolean` | Checks whether a value is an array. |

## Functional Methods

| Category | Methods |
| --- | --- |
| Iteration & Transformation | [`map()`](#map), [`filter()`](#filter), [`partition()`](#partition), [`reduce()`](#reduce), [`reduceRight()`](#reduceright), [`forEach()`](#foreach) |
| Windowing & Matrices | [`windows()`](#windows), [`scan()`](#scan), [`cartesianProduct()`](#cartesianproduct), [`zip()`](#zip), [`transpose()`](#transpose) |
| Deduplication & Grouping | [`unique()`](#unique), [`uniqueBy()`](#uniqueby), [`groupBy()`](#groupby) |
| Toggle | [`toggle()`](#toggle), [`toggleAll()`](#toggleall) |
| Search & Test | [`find()`](#find), [`findLast()`](#findlast), [`findIndex()`](#findindex), [`findLastIndex()`](#findlastindex), [`some()`](#some), [`every()`](#every), [`includes()`](#includes), [`equals()`](#equals), [`indexOf()`](#indexof), [`lastIndexOf()`](#lastindexof) |
| Flatten & Reorder | [`flat()`](#flat), [`flatMap()`](#flatmap), [`sort()`](#sort), [`toSorted()`](#tosorted), [`sortby()`](#sortby), [`sortedby()`](#sortedby), [`reverse()`](#reverse), [`rotate()`](#rotate), [`toReversed()`](#toreversed), [`random()`](#random), [`shuffle()`](#shuffle), [`sample()`](#sample) |
| Slicing | [`slice()`](#slice), [`splitAt()`](#splitat) |
| Aggregation | [`sum()`](#sum), [`product()`](#product), [`avg()`](#avg), [`sumBy()`](#sumby), [`avgBy()`](#avgby), [`count()`](#count), [`concat()`](#concat), [`join()`](#join) |
| Mutation | [`splice()`](#splice), [`fill()`](#fill), [`copyWithin()`](#copywithin) |
| Access & Iteration | [`first()`](#first), [`last()`](#last), [`at()`](#at), [`keys()`](#keys), [`values()`](#values), [`entries()`](#entries) |
| Immutable Copy | [`with()`](#with) |

### Iteration & Transformation

Predicate-based methods (`filter`, `partition`, `some`, `every`, `find`, `findLast`, `findIndex`, `findLastIndex`, `uniqueBy`, and `groupBy`) accept either an explicit arrow callback or an implicit expression such as `value > 2`; the element type is inferred from the receiver array.

#### `map()`

Creates a new array by applying a transformation callback to every element.

<SignatureBlock label="Signature">map&lt;U&gt;(callback: (element: T, index?: i64) =&gt; U): U[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-map.md -->

#### `filter()`

Selects elements that satisfy a predicate.

<SignatureBlock label="Signature">filter(predicate: (element: T) =&gt; boolean): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-filter.md -->

#### `partition()`

Splits the elements into two stable arrays according to a predicate. The first array contains matching elements; the second contains the remaining elements. The receiver is not mutated.

<SignatureBlock label="Signature">partition(predicate: (element: T) =&gt; boolean): [T[], T[]]</SignatureBlock>

<SignatureBlock label="Signature">partition(field: string): [T[], T[]]</SignatureBlock>

<SignatureBlock label="Signature">partition(field: string, ...values: string[]): [T[], T[]]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/partition.md -->

Object arrays can use a boolean field name directly, or a string field followed by one or more matching values. Field names are checked against the element type at compile time.

<!-- @include: ../.fragments/language/collections/functional-partition-properties.md -->

An unknown field is rejected at compile time.

::: danger error
<!-- @include: ../.fragments/language/collections/functional-partition-error.md -->
:::

For more complex predicates, use an explicit arrow function. It can receive the element, its index, and the original array:

<!-- @include: ../.fragments/language/collections/partition-2.md -->

`partition()` evaluates the predicate once per element and preserves the original order inside both output arrays. As with `count()`, a plain expression with exactly one free identifier is treated as an implicit per-element predicate; an explicit arrow function remains available for more complex cases. For memory safety, `T` must be `Copy`, `string`, or an object whose fields are `Copy` or `string`.

#### `reduce()`

Accumulates elements into a single value, from left to right.

<SignatureBlock label="Signature">reduce&lt;U&gt;(callback: (accumulator: U, element: T) =&gt; U, initial: U): U</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-reduce.md -->

#### `reduceRight()`

Accumulates elements into a single value, from right to left.

<SignatureBlock label="Signature">reduceRight&lt;U&gt;(callback: (accumulator: U, element: T) =&gt; U, initial: U): U</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-reduceRight.md -->

#### `forEach()`

Executes a callback for each element. The return value is discarded.

<SignatureBlock label="Signature">forEach(callback: (element: T) =&gt; void): void</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-forEach.md -->

### Search & Test

#### `find()`

Returns the first element matching a predicate, or absence if none match.

<SignatureBlock label="Signature">find(predicate: (element: T) =&gt; boolean): T?</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-find.md -->

#### `findLast()`

Returns the last element matching a predicate, or absence if none match.

<SignatureBlock label="Signature">findLast(predicate: (element: T) =&gt; boolean): T?</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-findLast.md -->

#### `findIndex()`

Returns the index of the first matching element, or `-1`.

<SignatureBlock label="Signature">findIndex(predicate: (element: T) =&gt; boolean): i64</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-findIndex.md -->

#### `findLastIndex()`

Returns the index of the last matching element, or `-1`.

<SignatureBlock label="Signature">findLastIndex(predicate: (element: T) =&gt; boolean): i64</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-findLastIndex.md -->

#### `some()`

Checks if at least one element satisfies a condition.

<SignatureBlock label="Signature">some(predicate: (element: T) =&gt; boolean): boolean</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-some.md -->

#### `every()`

Checks if all elements satisfy a condition.

<SignatureBlock label="Signature">every(predicate: (element: T) =&gt; boolean): boolean</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-every.md -->

#### `includes()`

Checks if a specific value exists in the array.

<SignatureBlock label="Signature">includes(value: T): boolean</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-includes.md -->

#### `equals()`

Compares two arrays element by element: the same length, the same elements, in the same order. `===` and `!==` do not compare arrays; the compiler reports `E0369` and points here.

<SignatureBlock label="Signature">equals(other: T[]): boolean</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-equals.md -->

#### `indexOf()`

Returns the first index of a value, or `-1` if not found.

<SignatureBlock label="Signature">indexOf(value: T, fromIndex?: i64): i64</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-indexOf.md -->

#### `lastIndexOf()`

Returns the last index of a value, or `-1` if not found.

<SignatureBlock label="Signature">lastIndexOf(value: T, fromIndex?: i64): i64</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-lastIndexOf.md -->

### Windowing, Scanning, and Matrix Operations

#### `windows()`

`windows(size)` returns overlapping copies of `size` consecutive elements. The receiver is unchanged; `windows(0)` is rejected and a size greater than the array length returns an empty array.

<!-- @include: ../.fragments/language/collections/windows.md -->

#### `scan()`

`scan(initial, callback)` returns the initial accumulator followed by each accumulated value. The callback receives the accumulator and the current element, and the receiver is unchanged.

<!-- @include: ../.fragments/language/collections/scan.md -->

#### `cartesianProduct()`

`cartesianProduct(other)` returns pairs in lexicographic receiver/argument order. Either empty input produces an empty result, and the receiver is unchanged.

<!-- @include: ../.fragments/language/collections/cartesianproduct.md -->

#### `zip()`

`zip(other)` interleaves the receiver and the argument into an array of pairs, `(T, U)[]`, where `T` is the receiver element type and `U` is the argument element type. The result length equals both inputs, which must match; the source arrays are left unchanged.

<!-- @include: ../.fragments/language/collections/zip.md -->

A length mismatch throws a recoverable `ZipError` (code `ERR_ARRAY_ZIP_MISMATCH`) instead of aborting, so the call must be covered by a `catch` or by a `throws ZipError` clause on the enclosing function:

<!-- @include: ../.fragments/language/collections/zip-2.md -->

#### `transpose()`

`transpose()` converts a rectangular matrix into columns. Empty matrices and matrices with zero-width rows return empty arrays; irregular rows fail before a partial result is produced.

<!-- @include: ../.fragments/language/collections/transpose.md -->

#### `unique()`

`unique()` returns the first occurrence of every distinct value in input order. It does not mutate the receiver and uses the native equality policy for strings and numbers.

<!-- @include: ../.fragments/language/collections/unique.md -->

#### `uniqueBy()`

`uniqueBy(selector)` keeps the first element for each distinct hashable key. The selector can be a field name for object elements, an explicit callback, or an implicit expression where the element is inferred automatically.

<!-- @include: ../.fragments/language/collections/uniqueby.md -->

#### `groupBy()`

`groupBy(selector)` creates a map from each hashable key to an array of elements in their original order. The selector can be a field name for object elements, an explicit callback, or an implicit expression where the element is inferred automatically. The first implementation requires Copy elements so each group owns its values safely.

<!-- @include: ../.fragments/language/collections/groupby.md -->

### Flatten & Reorder

#### `flat()`

Flattens nested arrays by one level.

<SignatureBlock label="Signature">flat(): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-flat.md -->

#### `flatMap()`

Maps each element to an array, then flattens the result by one level.

<SignatureBlock label="Signature">flatMap&lt;U&gt;(callback: (element: T) =&gt; U[]): U[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-flatMap.md -->

#### `sort()`

Sorts the array in place using a comparison function.

<SignatureBlock label="Signature">sort(compare?: (left: T, right: T) =&gt; i32): this</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-sort.md -->

#### `toSorted()`

Returns a new sorted copy without mutating the original.

<SignatureBlock label="Signature">toSorted(compare?: (left: T, right: T) =&gt; i32): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-toSorted.md -->

#### `sortBy()`

Sorts an array in place by a statically resolved field. The key must be a string literal naming a numeric, boolean, character, or string field. The optional second argument controls the order: it defaults to ascending and accepts `"desc"` for descending order.

<SignatureBlock label="Signature">sortBy(key: keyof T, order?: "asc" | "desc"): this</SignatureBlock>

<!-- @include: ../.fragments/language/collections/sortby.md -->

The field is resolved at compile time, so the generated stable merge-sort loop reads the field directly without runtime reflection. `sortBy()` requires a mutable `let` receiver and runs in `O(n log n)` time with `O(n)` auxiliary space.

#### `sortedBy()`

Returns a sorted copy by a statically resolved field and leaves the receiver unchanged. Its optional second argument defaults to ascending and accepts `"desc"` for descending order. The first version requires the element fields to be `Copy`, preventing an unsafe shallow copy of owned values. It uses stable `O(n log n)` merge sort with `O(n)` auxiliary space.

<SignatureBlock label="Signature">sortedBy(key: keyof T, order?: "asc" | "desc"): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/sortedby.md -->

#### `reverse()`

Reverses the array in place.

<SignatureBlock label="Signature">reverse(): this</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-reverse.md -->

#### `rotate()`

Rotates the array in place. Positive amounts rotate to the left; negative amounts rotate to the right. The amount is normalized by the array length, and empty or single-element arrays are unchanged.

<SignatureBlock label="Signature">rotate(amount: i64): this</SignatureBlock>

<!-- @include: ../.fragments/language/collections/rotate.md -->

`rotate()` requires a mutable `let` receiver and runs in `O(n)` time with `O(1)` auxiliary space, without allocating a second array.

#### `toReversed()`

Returns a new reversed copy without mutating the original.

<SignatureBlock label="Signature">toReversed(): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-toReversed.md -->

#### `random()`

Returns a uniformly selected `T`. An empty array aborts at runtime.

<SignatureBlock label="Signature">random(): T</SignatureBlock>

<!-- @include: ../.fragments/language/collections/random.md -->

::: danger error
<!-- @include: ../.fragments/language/collections/random-2.md -->

<!-- @include: ../.fragments/language/collections/random-3.md -->
:::

#### `shuffle()`

Returns a new uniformly shuffled array without mutating the receiver. Arrays with fewer than two elements abort at runtime.

<SignatureBlock label="Signature">shuffle(): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/shuffle.md -->

::: danger error
<!-- @include: ../.fragments/language/collections/shuffle-2.md -->

<!-- @include: ../.fragments/language/collections/shuffle-3.md -->
:::

#### `sample()`

Returns a new array with `count` uniformly selected elements. `count` must be between `1` and the receiver length, inclusive. Without options, source positions do not repeat. `{ repeat: true }` permits repeated positions but does not guarantee them.

<SignatureBlock label="Signature">sample(count: usize, options?: { repeat?: boolean }): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/sample.md -->

::: danger error
<!-- @include: ../.fragments/language/collections/sample-2.md -->

<!-- @include: ../.fragments/language/collections/sample-3.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/collections/sample-4.md -->

<!-- @include: ../.fragments/language/collections/sample-5.md -->
:::

### Slicing

#### `slice()`

Extracts a copy of a contiguous subarray.

<SignatureBlock label="Signature">slice(start?: i64, end?: i64): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-slice.md -->

Indexing with a `Range` is sugar for `.slice()`: `arr[a..b]` is `arr.slice(a, b)`, and every open-ended form works too, `arr[a..]`, `arr[..b]`, `arr[..=b]`, and the full-range `arr[..]`. Like `.slice()` itself, this always copies into a new array; tyrus arrays have no zero-copy borrowed-slice representation yet.

<!-- @include: ../.fragments/language/collections/range-slicing.md -->

#### `splitAt()`

Splits the array into two arrays at a given index.

<SignatureBlock label="Signature">splitAt(index: i64): [T[], T[]]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-splitAt.md -->

`splitAt` always returns two ordinary, dynamically sized arrays (`[T[], T[]]`), including when called on a fixed-size array. `index` may equal the array's length, yielding an empty second half, but a larger index is a compile error when the array's length is statically known, and a runtime abort otherwise.

### Aggregation

#### `count()`

Returns the length, or counts occurrences of a value or predicate match.

<SignatureBlock label="Signature">count(value?: T | (element: T) =&gt; boolean): i64</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-count.md -->

`count()` returns the length; `count(value)` counts elements equal to `value`; `count(condition)`, a plain expression referencing exactly one identifier not already declared in the current scope, is promoted to an implicit per-element condition, sugar for `count((implicitParam): boolean => condition)` with no arrow function to write. An identifier that already resolves in scope is not free, so the expression is checked as an ordinary `count(value)` call instead, comparing its (constant) `boolean` result against the element type is a compile error naming the identifier that shadowed the implicit parameter. An expression referencing more than one undeclared identifier is also a compile error, since the sugar only ever introduces one implicit parameter.

#### `sum()`

Returns the sum of all numeric elements using the array's element type. An empty numeric array returns that type's zero value.

<SignatureBlock label="Signature">sum(): T</SignatureBlock>

<!-- @include: ../.fragments/language/collections/sum.md -->

`sum()` is rejected for non-numeric element types and is compiled as a direct native loop without a callback or per-element runtime dispatch.

#### `product()`

Returns the product of all numeric elements using the array's element type. An empty numeric array returns that type's one value.

<SignatureBlock label="Signature">product(): T</SignatureBlock>

<!-- @include: ../.fragments/language/collections/product.md -->

`product()` follows the same numeric-type and native-loop rules as `sum()`.

#### `avg()`

Returns the arithmetic average of numeric elements. Integer, `f32`, and `f64` arrays return `f64?`; `f128` arrays return `f128?`. An empty array produces absence.

<SignatureBlock label="Signature">avg(): f64? | f128?</SignatureBlock>

<!-- @include: ../.fragments/language/collections/avg.md -->

The optional result must be handled with `??`, narrowing, or another valid optional operation before direct numeric use.

#### `sumBy()`

Returns the sum of a numeric field across all elements. The key must be a string literal naming a numeric field.

<SignatureBlock label="Signature">sumBy(key: string): T[K]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/sumby.md -->

#### `avgBy()`

Returns the arithmetic average of a numeric field across all elements. The key must be a string literal naming a numeric field. Always returns `f64?` (or `f128?` for `f128` fields).

<SignatureBlock label="Signature">avgBy(key: string): f64? | f128?</SignatureBlock>

<!-- @include: ../.fragments/language/collections/avgby.md -->

#### `concat()`

Merges two arrays into a new one.

<SignatureBlock label="Signature">concat(...items: (T | T[])[]): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-concat.md -->

#### `join()`

Joins all elements into a single string using a separator.

<SignatureBlock label="Signature">join(separator: string): string</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-join.md -->

### Mutation

#### `splice()`

Removes, replaces, or inserts elements in place.

<SignatureBlock label="Signature">splice(start: i64, deleteCount?: i64, ...items: T[]): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-splice.md -->

`splice(start, deleteCount?, ...items)` mutates the receiver and returns a new array containing the removed elements. Negative starts count from the end, deletion is clamped to the available range, and inserted values must match the array element type.

#### `fill()`

Replaces all elements with a fixed value.

<SignatureBlock label="Signature">fill(value: T): this</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-fill.md -->

#### `copyWithin()`

Copies a contiguous section of the array to another position in the same array.

<SignatureBlock label="Signature">copyWithin(target: i64, start: i64, end?: i64): this</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-copyWithin.md -->

### Access & Iteration

#### `first()`

Returns the first element as `T?`. An empty array produces absence. The type annotation is optional: when `values` is `i32[]`, tyrus infers `i32?` automatically. `Optional<i32>` is the equivalent explicit spelling.

<SignatureBlock label="Signature">first(): T?</SignatureBlock>

<!-- @include: ../.fragments/language/collections/first.md -->

`T?` is intentional: `first()` returns one value, not a collection, and remains `O(1)` without allocating an empty result array. Direct use as `i32` is rejected until an `if`, `??`, ternary, optional chaining, or another valid narrowing operation handles absence.

#### `last()`

Returns the last element as `T?`. An empty array produces absence, and tyrus infers the optional element type automatically when the annotation is omitted.

<SignatureBlock label="Signature">last(): T?</SignatureBlock>

<!-- @include: ../.fragments/language/collections/last.md -->

`last()` follows the same absence and narrowing rules as `first()`, while reading the final element in `O(1)` without allocating a result array.

#### `at()`

Returns the element at an index, supporting negative indexing from the end.

<SignatureBlock label="Signature">at(index: i64): T</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-at.md -->

#### `keys()`

Returns an iterator over the array indices.

<SignatureBlock label="Signature">keys(): Iterable&lt;i64&gt;</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-keys.md -->

#### `values()`

Returns an iterator over the array values.

<SignatureBlock label="Signature">values(): Iterable&lt;T&gt;</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-values.md -->

#### `entries()`

Returns an iterator over index-value pairs.

<SignatureBlock label="Signature">entries(): Iterable&lt;[i64, T]&gt;</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-entries.md -->

### Immutable Copy

#### `with()`

Returns a new array with a single element replaced at a given index.

<SignatureBlock label="Signature">with(index: i64, value: T): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/functional-with.md -->

### Toggle

#### `toggle()`

Adds an element to the array if it is not present, and removes it if it is present, returning a new array and never mutating the receiver.

<SignatureBlock label="Signature">toggle(item: T): T[]</SignatureBlock>

<SignatureBlock label="Signature">toggle(item: T, key: string): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/toggle.md -->

When a `key` field name is given, matching is done by that field instead of by the element as a whole. The field must exist on the element type.

<!-- @include: ../.fragments/language/collections/toggle-2.md -->

#### `toggleAll()`

Applies the same add-or-remove behavior to every element of the argument array, preserving order. Mutates nothing.

<SignatureBlock label="Signature">toggleAll(items: T[]): T[]</SignatureBlock>

<!-- @include: ../.fragments/language/collections/toggleall.md -->

### Chaining

Functional methods chain into readable data pipelines:

<!-- @include: ../.fragments/language/collections/functional-chaining.md -->
