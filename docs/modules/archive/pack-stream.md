---
title: packStream
description: Write an archive into a byte stream as the source is read.
---

# packStream

`packStream` returns a `ReadableStream<u8[]>` carrying the archive of a file or directory as it is written, in the format you name: nothing is written to disk, so the archive can go straight into an HTTP response, a compression stream, or a file. The stream holds at most 64 KiB, and the archive is produced only as fast as it is read.

ZIP is written with data descriptors after each entry, since a stream cannot go back to fill sizes in; every current tool reads it.

<SignatureBlock label="Signature">packStream(source: string, format: ArchiveFormat, options?: PackOptions): ReadableStream&lt;u8[]&gt;</SignatureBlock>

## Minimal example

<!-- @include: ../../.fragments/modules/archive/pack-stream/minimal.md -->

## Every option

<!-- @include: ../../.fragments/modules/archive/pack-stream/maximal.md -->

`overwrite` has no effect, since there is no destination. A failure, from a missing source to the time limit, fails the stream with the [`ArchiveError`](./types.md#archiveerror) instead of ending it, so a reader never mistakes a truncated archive for a complete one: `read()` and `pipeTo()` reject, and `catch (error: ArchiveError)` receives it.
