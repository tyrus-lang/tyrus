---
title: pack
description: Create a ZIP, TAR, or TAR.GZ archive from a file or directory.
---

# pack

`pack` creates an archive from a file or a directory. The destination's lowercase extension decides the format: `.zip`, `.tar`, `.tar.gz`, or `.tgz`.

Entries are written in a stable order with `/` separators, and the archive is first written to a temporary file next to the destination, then renamed into place, so a failure never leaves a truncated archive behind. When the destination lies inside the source directory, it is left out of the archive.

<SignatureBlock label="Signature">pack(source: string, destination: string, options?: PackOptions): Promise&lt;Result&lt;ArchiveSummary, ArchiveError&gt;&gt;</SignatureBlock>

## Minimal example

<!-- @include: ../../.fragments/modules/archive/pack/minimal.md -->

## Every option

<!-- @include: ../../.fragments/modules/archive/pack/maximal.md -->

| Option | Type | Default | Effect |
|--------|------|---------|--------|
| `level` | `i32` | `6` | Compression level from `0` (store only) to `9` (smallest output). Not accepted for `.tar`. |
| `overwrite` | `boolean` | `false` | Replaces an existing destination file. A directory is never replaced. |
| `exclude` | `string[]` | none | Glob patterns, relative to the source, for files and directories to leave out. `**` spans any number of directories. |
| `prefix` | `string` | none | Directory that holds every entry, such as `release-1.0.0`. |
| `reproducible` | `boolean` | `false` | Writes fixed timestamps and normalized permissions, so the same input produces the same bytes. |
| `timeoutMs` | `u64` | `600000` (10 minutes) | Gives up with `ERR_ARCHIVE_TIMEOUT` when packing takes longer, leaving no partial archive behind. `0` fails with `ERR_ARCHIVE_INVALID_OPTION`. |

Symbolic links and special files in the source are refused with `ERR_ARCHIVE_UNSUPPORTED_ENTRY`, which names the offending path in `entry`, instead of being silently skipped.

## Error Handling

::: danger error
<!-- @include: ../../.fragments/modules/archive/pack/error-handling.md -->
:::

A literal destination with an unknown or uppercase extension, and a literal `level` the format cannot use, are reported by the compiler before the program runs.
