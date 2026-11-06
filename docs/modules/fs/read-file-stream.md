---
title: readFileStream
description: Read a file progressively as bounded binary chunks.
---

# readFileStream

`readFileStream` opens a file and returns a bounded `Result<ReadableStream<u8[]>, FsError>`. The runtime reads chunks progressively, so memory use does not grow with the complete file size.

## Signature

<!-- @include: ../../.fragments/modules/fs/read-file-stream/signature.md -->

The default chunk size is 64 KiB and the bounded queue holds at most 256 KiB. Each chunk preserves the original bytes, including zero bytes and data that is not valid text.

## Example

<!-- @include: ../../.fragments/modules/fs/read-file-stream/basic-usage.md -->

Opening failures return `Err(FsError)`. Failures after the file is open reject stream operations with `StreamError`.
