---
title: expect
description: Check a value with a matcher.
---

# expect

<SignatureBlock label="Signature">expect&lt;T&gt;(value: T): Assertion</SignatureBlock>

`expect(value)` evaluates `value` exactly once and returns an assertion. A matcher called on the assertion compares the value and fails the test when the comparison does not hold. `.not` before the matcher inverts it.

| Matcher | Passes when |
| --- | --- |
| `toBe(expected)`, `toEqual(expected)`, `toStrictEqual(expected)` | the values are equal: a scalar, an object field by field, or an array element by element, all the way down |
| `toBeTruthy()`, `toBeFalsy()` | the value is truthy or falsy |
| `toBeNull()`, `toBeUndefined()`, `toBeDefined()` | an optional value is absent or present |
| `toBeGreaterThan(n)`, `toBeGreaterThanOrEqual(n)` | the number is above `n` (or equal to it) |
| `toBeLessThan(n)`, `toBeLessThanOrEqual(n)` | the number is below `n` (or equal to it) |
| `toBeCloseTo(n)` | the float is within `0.005` of `n` |
| `toHaveLength(n)` | the string or array has length `n` |
| `toContain(value)` | the string contains the substring, or the array contains the element |
| `toMatch(text)` | the string contains `text` |
| `toThrow()`, `toThrow(message)` | calling the function throws (and the thrown `string` equals `message`); the function must declare `throws` and take no arguments |

A failed matcher names the values that were compared, with strings in quotes: `expected "abc" to equal "abd"`.

Two matchers other frameworks have are deliberately left out, and writing one is a compile error that names what to use instead. `toHaveProperty` asks something the compiler has already settled: a field is resolved at compile time, so the assertion can only ever be true; to check whether an optional field holds a value, write `expect(user.email).toBeDefined()`. `toBeInstanceOf` has nothing to consult, since there are no class instances; narrow the value with `match` or an optional check and assert on what the narrowed value holds.

`.rejects` before `toThrow` settles a promise and asserts that it failed: `expect(load()).rejects.toThrow("not found")`, with the message optional and `.not` available. It settles through the runtime instead of suspending, so it works in a synchronous test too. The promise must be one whose failure is a rejection, such as `Promise.reject(...)` or an `async` function that returns one; a function that declares `throws` has that effect handled where it is called, before the matcher sees it. `.resolves` settles a promise the same way and hands the matcher that follows the value it resolved to, so every matcher works after it: `expect(load()).resolves.toBe(42)`, `expect(load()).resolves.toEqual({ id: 1 })`. A promise that failed instead fails the assertion, naming what it rejected with. The two are exclusive, since a promise cannot be asserted to both fail and succeed, and `toThrow` belongs to `.rejects`: after `.resolves` the matcher already has the value, so there is no failure left to assert. `toBe`, `toEqual` and `toStrictEqual` compare scalar values, objects field by field (the comparison the automatic `equals` performs), and arrays element by element. An array of scalars uses the element equality `includes` and `indexOf` use; an array of objects, or of arrays, is compared all the way down, stopping at the first difference.

<!-- @include: ../../.fragments/modules/test/expect.md -->
