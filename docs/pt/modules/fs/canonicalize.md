---
title: canonicalize
description: Resolve um caminho para um caminho absoluto.
---

# canonicalize

Resolve um caminho para um caminho absoluto e normalizado, resolvendo todos os links simbólicos. Retorna `FsError` se o caminho não existir.

<SignatureBlock label="Assinatura">canonicalize(path: string): Result&lt;string, FsError&gt;</SignatureBlock>

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/canonicalize/example.md -->
