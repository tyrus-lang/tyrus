---
title: unpack
description: Safely extract a ZIP, TAR, or TAR.GZ archive into a directory.
---

# unpack

`unpack` extracts a ZIP, TAR, or TAR.GZ archive into a destination directory, creating it when it does not exist. The format is identified from the content, never from the file name.

<SignatureBlock label="Signature">unpack(archive: string, destination: string, options?: UnpackOptions): Promise&lt;Result&lt;ArchiveSummary, ArchiveError&gt;&gt;</SignatureBlock>

## Minimal example

<!-- @include: ../../.fragments/modules/archive/unpack/minimal.md -->

## Every option

<!-- @include: ../../.fragments/modules/archive/unpack/maximal.md -->

| Option | Type | Default | Effect |
|--------|------|---------|--------|
| `overwrite` | `boolean` | `false` | Replaces files that already exist in the destination. Directories and links are never replaced. |
| `stripComponents` | `u32` | `0` | Drops that many leading path components from every entry; entries left empty are skipped. |
| `maxEntries` | `u64` | `100000` | Rejects archives with more entries. |
| `maxBytes` | `u64` | `4 GiB` | Rejects extraction that would write more bytes, counted as they are written. |
| `maxRatio` | `u32` | `200` | Rejects a ZIP entry that expands more than this many times its compressed size, and a TAR.GZ archive whose written bytes exceed this many times the compressed bytes read so far, once more than 1 MiB has been written. |
| `timeoutMs` | `u64` | `600000` (10 minutes) | Gives up with `ERR_ARCHIVE_TIMEOUT` when the operation takes longer, removing what it created. |

Limits can be raised, but never disabled: `0` fails with `ERR_ARCHIVE_INVALID_OPTION`.

## Safety

- Every entry name is validated before anything is created: absolute paths, `..` components, drive prefixes, backslashes, and names some platform cannot create, such as `CON` or names ending in a dot, are refused.
- Writes stay inside the destination even when it already contains a symbolic link: extraction never passes through a link.
- Symbolic links, hard links, devices, and other special entries are refused instead of being recreated.
- Two entries that resolve to the same path, including names that differ only in case or Unicode normalization, are refused.
- Each file is written to a temporary name and renamed when complete, and on any failure the files and directories this call created are removed. Content that already existed in the destination is left untouched.
- Extracted files never receive special permission bits; only the owner's execute permission is kept, so extracted programs remain runnable.
- Before a ZIP archive is extracted, the sizes it declares, capped at `maxBytes`, are compared with the free space in the destination, and an archive that cannot fit is refused with `ERR_ARCHIVE_NO_SPACE` before any file is created. Sizes are never trusted beyond that: the limits count the bytes actually written, and a disk that fills up during extraction also rejects with `ERR_ARCHIVE_NO_SPACE`.
- Archives inside the archive are never extracted: a `.zip` or `.tar.gz` entry is written as an ordinary file.
- `unpack` never runs, opens, or interprets anything it extracts.

## Error Handling

::: danger error
<!-- @include: ../../.fragments/modules/archive/unpack/error-handling.md -->
:::
