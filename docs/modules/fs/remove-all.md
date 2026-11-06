---
title: removeAll
description: Recursively removes a directory and all its contents.
---

# removeAll

Recursively removes a directory and all of its contents. This is equivalent to `rm(path, { recursive: true })` but does not accept options.

<SignatureBlock label="Signature">removeAll(path: string): Result&lt;void, FsError&gt;</SignatureBlock>

The operation returns `Err(FsError)` on failure; inspect it with `match` or propagate it with `?`.

## Example

<!-- @include: ../../.fragments/modules/fs/remove-all/basic-usage.md -->
