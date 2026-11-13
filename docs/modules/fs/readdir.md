---
title: readdir
description: Reads the contents of a directory.
---

# readdir

Reads the contents of a directory and returns an array of `DirEntry` objects.

<SignatureBlock label="Signature">readdir(path: string, options?: ReaddirOptions): Result&lt;DirEntry[], FsError&gt;</SignatureBlock>

## Read options

`ReaddirOptions` is an interface with one optional field:

- `recursive?: boolean`, when `true`, recursively traverses subdirectories and includes their entries in the result. Defaults to `false`.

## Directory entries

Each `DirEntry` has readonly `name: string` and `type: DirEntryType` fields. `DirEntryType` is an enum with `File`, `Directory`, `Symlink`, and `Other` members, so callers can branch on an entry's kind without inspecting its name.

## Example

<!-- @include: ../../.fragments/modules/fs/readdir/basic-usage.md -->

## Error handling

Returns `Err(FsError)` with code `ERR_FS_READ_FAILED` if the root directory or any directory traversed recursively cannot be read, or if an entry cannot be inspected. The operation does not return a partial listing.

::: danger error
<!-- @include: ../../.fragments/modules/fs/readdir/error-handling.md -->
:::
