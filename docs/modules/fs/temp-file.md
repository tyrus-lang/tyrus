---
title: tempFile
description: Creates a temporary file and returns its path.
---

# tempFile

Creates a new temporary file and returns its path. The file exists (empty) on return and is not automatically cleaned up.

<SignatureBlock label="Signature">tempFile(): Result&lt;string, FsError&gt;</SignatureBlock>

## Example

<!-- @include: ../../.fragments/modules/fs/temp-file/example.md -->
