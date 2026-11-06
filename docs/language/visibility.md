---
title: Visibility
description: The three levels of visibility, no export, export, and export protect, plus how a re-export and a namespace import behave.
---

# Visibility

Visibility answers one question: which files may import a name. tyrus has three levels, and a declaration picks one of them by what it writes in front of itself.

| Form | Who may import it |
|---|---|
| no `export` | nobody, the name exists only inside its own file |
| `export` | every file in the project |
| `export protect` | the folder that declares it, and every subfolder of that folder |

The three levels are a chain: `export protect` sits between a name nobody can reach and a name everybody can reach, which is the level a component needs to share a helper between its own files without publishing it.

## No `export`: private to the file

A declaration without `export` is invisible outside its file. There is no keyword for it, since it is the default.

<!-- @include: ../.fragments/language/visibility/file-private.md -->

Importing it is an error: as far as any other file is concerned, the name does not exist.

::: danger error
<!-- @include: ../.fragments/language/visibility/file-private-error.md -->
:::

## Visibility controls the import, not the call

Visibility decides which files may *name* a declaration. It says nothing about which declarations may *use* it. A public function is free to call a private helper in its own file, and importing that function works even though the helper is unreachable from the importer.

<!-- @include: ../.fragments/language/visibility/call-through.md -->

`sign` is public, so `src/http/handler.tyr` may import it. `salt` is private to `src/auth/token.tyr`, so the same file may not import it, and does not need to: the call to `salt` lives inside `token.tyr`, where the name is in scope. The same holds one level up, with `export protect salt` instead of a private `salt`: a public `sign` may still call it, and a file outside the folder may still import `sign`.

This is what makes the levels useful. A component publishes the small surface it wants callers to see and keeps the machinery behind it at whatever level fits, without having to widen the machinery to match the surface.

## `export`: public in the project

`export` publishes a name to every file in the project, whatever folder it sits in.

<!-- @include: ../.fragments/language/visibility/public.md -->

The reach stops at the project. The compiler refuses an import that resolves outside the project root, so `export` means "public within this project", never "public to the world". External packages are not supported yet.

## `export protect`: the declaring folder and below

`export protect` narrows the reach to the folder that declares the name, plus every subfolder of that folder.

<!-- @include: ../.fragments/language/visibility/protect.md -->

Both importers above are inside `src/auth/`, so both resolve. A file in a sibling folder is not, and gets `E3308` naming the symbol, the folder it is limited to, and the two ways out.

::: danger error
<!-- @include: ../.fragments/language/visibility/protect-error.md -->
:::

The scope depends only on the declaring file, never on the depth of `src/`, so moving a whole folder preserves every relation inside it.

### A nested folder is a narrower boundary

The relation runs downward only. A name protected in a subfolder is not visible in the folder above it, because the importer has to be a descendant of the folder that declares.

<!-- @include: ../.fragments/language/visibility/protect-nested.md -->

::: danger error
<!-- @include: ../.fragments/language/visibility/protect-nested-error.md -->
:::

Each folder is a real boundary, not a mutual club. If a nested file has to be reachable from the whole component, declare it with a plain `export` and accept the wider reach.

### On a separate clause

The modifier also goes on an export clause, which is how a file publishes some names narrowly and others openly.

<!-- @include: ../.fragments/language/visibility/protect-clause.md -->

## A re-export cannot widen the scope

A re-export forwards a name without declaring it, so it carries the original scope unchanged. Inside the folder, going through a barrel file works as usual.

<!-- @include: ../.fragments/language/visibility/protect-reexport.md -->

Outside the folder it does not, because the barrel forwarded the scope along with the name. Neither `export *` nor `export { name } from` is a way out.

::: danger error
<!-- @include: ../.fragments/language/visibility/protect-reexport-error.md -->
:::

## A namespace import omits protected names

A namespace import binds every export at once. Names out of scope are simply absent from the namespace rather than bound and then refused.

<!-- @include: ../.fragments/language/visibility/protect-namespace.md -->

Reaching for `token.sign` from `src/main.tyr` fails the same way any name the module does not export fails.

## What `protect` rejects

`protect` describes where a declaration is visible, so it needs a declaration to attach to. Two forms do not have one.

| Form | Code | Why |
|---|---|---|
| `export protect default name;` | `E3309` | a default export publishes an anonymous slot, so folder scope has nothing to attach to |
| `export protect { name } from "./other";` | `E3310` | a re-export does not declare the name, it forwards one, and forwarding keeps the original scope |

Writing the modifier first, as `protect export`, is a plain syntax error. The order is fixed at `export protect`, so there is only one spelling.

## Tests and the source root

There is no exemption for test files. A test that needs a protected name belongs inside the folder that owns it, colocated with the code it tests.

A `protect` declaration in the source root narrows nothing, since its folder already covers every file the compiler would accept. That is valid but pointless, and the `export-protect-no-effect` lint rule warns about it.
