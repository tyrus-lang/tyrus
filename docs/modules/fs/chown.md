---
title: chown
description: Changes file ownership.
---

# chown

Changes the owner and group of a file or directory. On non-Unix platforms, this returns `Err` with code `ERR_FS_CHOWN_FAILED`.

<SignatureBlock label="Signature">chown(path: string, uid: u32, gid: u32): Result&lt;void, FsError&gt;</SignatureBlock>

## Parameters

- `path`, the file path to change.
- `uid`, the new user ID.
- `gid`, the new group ID.

## Example

<!-- @include: ../../.fragments/modules/fs/chown/example.md -->
