---
title: mime
description: Gets MIME type from file extension.
---

# mime

Returns the MIME type for a file based on its extension. Returns no value if the extension is unknown or the path has no extension. Unlike operations that return `Result`, `mime` has no recoverable failure case.

<SignatureBlock label="Signature">mime(path: string): string?</SignatureBlock>

## Example

<!-- @include: ../../.fragments/modules/fs/mime/example.md -->
