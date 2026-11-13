---
title: removeAll
description: Remove recursivamente um diretório e todo o seu conteúdo.
---

# removeAll

Remove recursivamente um diretório e todo o seu conteúdo. É equivalente a `rm(path, { recursive: true })`, mas não aceita opções.

<SignatureBlock label="Assinatura">removeAll(path: string): Result&lt;void, FsError&gt;</SignatureBlock>

A operação retorna `Err(FsError)` quando falha; inspecione com `match` ou propague com `?`.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/remove-all/basic-usage.md -->
