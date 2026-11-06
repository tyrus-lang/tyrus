---
title: hardLink
description: Creates a hard link.
---

# hardLink

Creates a hard link at `link` pointing to `original`. Hard links share the same inode as the original file.

<SignatureBlock label="Signature">hardLink(original: string, link: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Example

<!-- @include: ../../.fragments/modules/fs/hard-link/example.md -->
