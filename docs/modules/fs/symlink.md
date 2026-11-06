---
title: symlink
description: Creates a symbolic link.
---

# symlink

Creates a symbolic link at `link` pointing to `original`. On Unix, `original` can be a file or directory. On Windows, this operation creates file symlinks; use [`symlinkDir`](./symlink-dir.md) for directory symlinks. Creating a Windows symlink may require Developer Mode, `SeCreateSymbolicLinkPrivilege`, or an elevated process; see [Microsoft's `CreateSymbolicLinkW` documentation](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinkw). Failures are returned as `Err(FsError)`.

<SignatureBlock label="Signature">symlink(original: string, link: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Example

<!-- @include: ../../.fragments/modules/fs/symlink/example.md -->
