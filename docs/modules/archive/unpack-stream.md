---
title: unpackStream
description: Extract a TAR or TAR.GZ archive from a byte stream as it arrives.
---

# unpackStream

`unpackStream` extracts a TAR or TAR.GZ archive from a `ReadableStream<u8[]>` into a directory while the bytes arrive, with the same safety as [`unpack`](./unpack.md): entries stay inside the destination, links and special files are refused, the limits apply, and on failure everything the call created is removed. The format is identified from the first bytes.

A ZIP archive cannot be extracted from a stream: its entry table is at the end, and the entries before it cannot be trusted until it is read. Such a stream fails with `ERR_ARCHIVE_ZIP_NOT_STREAMABLE`; write it to a file and use `unpack`.

<SignatureBlock label="Signature">unpackStream(archive: ReadableStream&lt;u8[]&gt;, destination: string, options?: UnpackOptions): Promise&lt;Result&lt;ArchiveSummary, ArchiveError&gt;&gt;</SignatureBlock>

## Minimal example

<!-- @include: ../../.fragments/modules/archive/unpack-stream/minimal.md -->

## Every option

<!-- @include: ../../.fragments/modules/archive/unpack-stream/maximal.md -->

The stream is borrowed: the call reads it to the end of the archive, and the program keeps its handle. A stream that fails while being read resolves with `ERR_ARCHIVE_IO`, and one nobody writes to fails at `timeoutMs`.
