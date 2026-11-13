---
title: list
description: Read the entries of an archive without extracting it.
---

# list

`list` reads the entries of a ZIP, TAR, or TAR.GZ archive without writing anything to disk. The format is identified from the content, never from the file name.

Use it to inspect an untrusted archive before extracting it: entries that `unpack` would refuse, such as links, are listed with their kind, and names are reported exactly as the archive stores them.

<SignatureBlock label="Signature">list(archive: string, options?: UnpackOptions): Promise&lt;Result&lt;ArchiveEntry[], ArchiveError&gt;&gt;</SignatureBlock>

## Minimal example

<!-- @include: ../../.fragments/modules/archive/list/minimal.md -->

## Every option

<!-- @include: ../../.fragments/modules/archive/list/maximal.md -->

`list` reads only `maxEntries` and `timeoutMs` from [`UnpackOptions`](./types.md#unpackoptions); the other options apply to extraction. An archive with more entries fails with `ERR_ARCHIVE_ENTRY_LIMIT`, and a ZIP archive fails before its entry table is read.
