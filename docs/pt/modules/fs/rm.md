---
title: rm
description: Remove um arquivo ou diretório.
---

# rm

Remove um arquivo ou diretório. Para diretórios, use `{ recursive: true }` para remover todo o conteúdo.

<SignatureBlock label="Assinatura">rm(path: string, options?: RmOptions): Result&lt;void, FsError&gt;</SignatureBlock>

A operação retorna `Err(FsError)` quando falha; inspecione com `match` ou propague com `?`.

## Opções

`RmOptions` é uma interface com dois campos opcionais:

- `recursive?: boolean`: quando `true`, remove diretórios e todo o conteúdo recursivamente. O padrão é `false`.
- `force?: boolean`: quando `true`, ignora caminhos inexistentes sem emitir erro. O padrão é `false`.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/rm/basic-usage.md -->
