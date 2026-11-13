---
title: symlinkDir
description: Creates a symbolic link to a directory.
---

# symlinkDir

Creates a symbolic link at `link` pointing to the directory at `original`. This operation identifies the link as a directory link even when the target does not exist yet. On Windows, creating a symbolic link may require Developer Mode, `SeCreateSymbolicLinkPrivilege`, or an elevated process; see [Microsoft's `CreateSymbolicLinkW` documentation](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinkw). Failures are returned as `Err(FsError)`.

<SignatureBlock label="Signature">symlinkDir(original: string, link: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Example

<!-- @include: ../../.fragments/modules/fs/symlink-dir/example.md -->
