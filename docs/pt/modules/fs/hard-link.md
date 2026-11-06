---
title: hardLink
description: Cria um link físico.
---

# hardLink

Cria um link físico em `link` que aponta para `original`. Links físicos compartilham o mesmo inode do arquivo original.

<SignatureBlock label="Assinatura">hardLink(original: string, link: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/hard-link/example.md -->
