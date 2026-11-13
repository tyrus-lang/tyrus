---
title: canonicalize
description: Resolves a path to an absolute path.
---

# canonicalize

Resolves a path to an absolute, normalized path with all symlinks resolved. Throws `FsError` if the path does not exist.

<SignatureBlock label="Signature">canonicalize(path: string): Result&lt;string, FsError&gt;</SignatureBlock>

## Example

<!-- @include: ../../.fragments/modules/fs/canonicalize/example.md -->
