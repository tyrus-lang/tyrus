---
title: atomicWrite
description: Writes a file atomically.
---

# atomicWrite

Writes data to a file atomically by first writing to a unique temporary file in the target directory and then replacing the target. This prevents partial writes in case of failure and works when the target already exists on Windows, macOS, and Linux.

<SignatureBlock label="Signature">atomicWrite(path: string, data: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Example

<!-- @include: ../../.fragments/modules/fs/atomic-write/example.md -->
