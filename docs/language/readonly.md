---
title: Readonly
description: The six readonly forms, interface properties, type position, parameters, Readonly<T>, as const, and readonly const.
---

# Readonly

`readonly` is about *mutability*: it says a value cannot be written through, at any depth. It is not about *rebinding*, which is what `const` and `let` decide, and it is not about *narrowing a type to a literal*, which is what `as const` adds on top.

The keyword appears in six distinct places. They all produce the same guarantee, nothing behind this value can be assigned to, and differ only in what they attach to.

| Form | Where it goes | What it protects |
|---|---|---|
| `readonly id: i32` | interface property | that one property, and everything under it |
| `readonly Config` | type position | every value of that type |
| `config: readonly Config` | parameter | the argument, for the duration of the call |
| `Readonly<Config>` | type utility | every property of an interface at once |
| `[1, 2] as const` | expression suffix | the value, and it narrows each element to its literal type |
| `readonly const PRIMES: i32[]` | declaration | the binding, deeply |

The guarantee is recursive in every one of them. A `readonly` value one level down is still `readonly`, so a nested write is refused just as the surface write is.

## `readonly` on an interface property

The narrowest form: one property of one interface. Everything else on the interface stays writable.

::: danger error
<!-- @include: ../.fragments/language/readonly/interface-property.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/readonly/interface-property-diagnostic.md -->
:::

## `readonly` in type position

Written before a type, `readonly` produces the readonly view of it. On an array this removes the mutating methods.

::: danger error
<!-- @include: ../.fragments/language/readonly/type-position.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/readonly/type-position-diagnostic.md -->
:::

A readonly value also cannot be laundered into a mutable binding, which would hand out exactly the write access the type refused.

::: danger error
<!-- @include: ../.fragments/language/readonly/type-position-transfer.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/readonly/type-position-transfer-diagnostic.md -->
:::

## `readonly` on a parameter

A parameter takes `readonly` in the *type* position, not before the name. `readonly config: Config` is rejected; `config: readonly Config` is the form.

<!-- @include: ../.fragments/language/readonly/parameter.md -->

This is the signature to reach for when a function reads a value it did not create: the caller can see from the signature alone that its value comes back untouched.

::: danger error
<!-- @include: ../.fragments/language/readonly/parameter-mutation.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/readonly/parameter-mutation-diagnostic.md -->
:::

## `Readonly<T>`

`Readonly<T>` applies the qualifier to every property of an interface at once, instead of one property at a time. It is recursive, including for nested objects and arrays. The full description is on the [Objects and Interfaces](/language/objects#readonly-t) page.

## `as const`

`as const` does two things, and only one of them is about mutability. It freezes the value, and it narrows each element to its own literal type: in `[3000, 3001] as const` the first element has the type `3000`, not `i32`.

<!-- @include: ../.fragments/language/readonly/as-const.md -->

::: tip `as const` does not replace `readonly`
Use `readonly` when the point is that nothing may be written. Use `as const` when the point is that the *exact literal values* are part of the type. The freezing that comes with `as const` is a consequence of that narrowing, not its purpose.
:::

## `readonly const`

`readonly const` is the declaration form. The binding is never rebound, because it is a `const`, and nothing behind it can be written, because it is `readonly`.

<!-- @include: ../.fragments/language/readonly/readonly-const.md -->

A global `readonly const` whose initializer is a literal table costs nothing at run time: it is emitted into read-only memory, with no allocation, no element stores and nothing to release.

`readonly const x: T` and `const x: readonly T` are the same declaration, so the formatter settles on the first spelling and writes both that way. The one place it does not is a `for` initializer, where the grammar admits only `const` and `let`.

The keyword goes before `const`, and the order is checked.

::: danger error
<!-- @include: ../.fragments/language/readonly/readonly-const-order.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/readonly/readonly-const-order-diagnostic.md -->
:::

A anotação é obrigatória, porque não há mais nada a que o qualificador possa se prender.

::: danger error
<!-- @include: ../.fragments/language/readonly/readonly-const-annotation.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/readonly/readonly-const-annotation-diagnostic.md -->
:::

`readonly` qualifies `const` only. A `let` can be rebound, so a readonly `let` would promise something the next line could take away, and it is rejected.

## `readonly` is not `const`

The two are easy to confuse because a `const` binding is already deeply immutable in tyrus. The difference shows up as soon as the value travels.

`const` is a property of the *binding*: it stops that name from being rebound, and stops writes through that name. It says nothing about the type, so passing the value to a function that takes a mutable parameter is fine.

`readonly` is a property of the *type*: it travels with the value into parameters, return types, fields and other bindings. That is what makes it usable in a signature, and what makes a `readonly const` different from a plain `const`, the protection survives the value being handed on.
