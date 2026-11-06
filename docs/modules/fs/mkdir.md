---
title: mkdir
description: Creates a new directory.
---

# mkdir

Creates a new directory at the given path. The path is resolved transparently before reaching the operating system. It returns `Err(FsError)` if creation fails; inspect the result with `match` or propagate it with `?`.

<SignatureBlock label="Signature">mkdir(path: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Example

<!-- @include: ../../.fragments/modules/fs/mkdir/basic-usage.md -->
