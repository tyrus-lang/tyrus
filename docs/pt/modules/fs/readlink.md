---
title: readlink
description: Lê o destino de um link simbólico.
---

# readlink

Retorna o caminho de destino para o qual um link simbólico aponta ou `ERR_FS_READLINK_FAILED` se o caminho não for um link simbólico legível. Um caminho inválido retorna `ERR_FS_INVALID_PATH`.

<SignatureBlock label="Assinatura">readlink(path: string): Result&lt;string, FsError&gt;</SignatureBlock>

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/readlink/example.md -->
