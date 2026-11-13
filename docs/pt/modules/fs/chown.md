---
title: chown
description: Altera o proprietário de um arquivo.
---

# chown

Altera o proprietário e o grupo de um arquivo ou diretório. Em plataformas que não são Unix, retorna `Err` com o código `ERR_FS_CHOWN_FAILED`.

<SignatureBlock label="Assinatura">chown(path: string, uid: u32, gid: u32): Result&lt;void, FsError&gt;</SignatureBlock>

## Parâmetros

- `path`: caminho do arquivo a alterar.
- `uid`: novo ID de usuário.
- `gid`: novo ID de grupo.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/chown/example.md -->
