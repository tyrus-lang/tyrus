---
title: writeFileStream
description: Write bounded binary chunks progressively to a file.
---

# writeFileStream

`writeFileStream` creates or truncates a file and returns a bounded `Result<WritableStream<u8[]>, FsError>`. Backpressure keeps producers from buffering the complete output in memory.

## Signature

<!-- @include: ../../.fragments/modules/fs/write-file-stream/signature.md -->

The queue holds at most 256 KiB. `await stream.close()` and `await source.pipeTo(stream)` resolve only after all accepted chunks have been written and the file buffer has been flushed.

## Example

<!-- @include: ../../.fragments/modules/fs/write-file-stream/basic-usage.md -->

Opening failures return `Err(FsError)`. Write or flush failures reject stream operations with `StreamError`.
