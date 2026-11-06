---
title: readlink
description: Reads a symbolic link target.
---

# readlink

Returns the target path that a symbolic link points to, or `ERR_FS_READLINK_FAILED` if the path is not a readable symbolic link. An invalid path returns `ERR_FS_INVALID_PATH`.

<SignatureBlock label="Signature">readlink(path: string): Result&lt;string, FsError&gt;</SignatureBlock>

## Example

<!-- @include: ../../.fragments/modules/fs/readlink/example.md -->
