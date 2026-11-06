---
title: mv
description: Moves (renames) a file or directory.
---

# mv

Moves (renames) a file or directory from `src` to `dest` on the same filesystem. Cross-device moves return `ERR_FS_MV_FAILED`; the source is left in place.

<SignatureBlock label="Signature">mv(src: string, dest: string): Result&lt;void, FsError&gt;</SignatureBlock>

The operation returns `Err(FsError)` on failure; inspect it with `match` or propagate it with `?`.

## Example

<!-- @include: ../../.fragments/modules/fs/mv/basic-usage.md -->
