---
title: hardLink
description: Crea un enlace duro.
---

# hardLink

Crea un enlace duro en `link` que apunta a `original`. Los enlaces duros comparten el mismo inode que el archivo original.

<SignatureBlock label="Firma">hardLink(original: string, link: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/hard-link/example.md -->
