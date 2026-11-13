---
title: mv
description: Move (renomeia) um arquivo ou diretório.
---

# mv

Move (renomeia) um arquivo ou diretório de `src` para `dest` no mesmo sistema de arquivos. Movimentos entre dispositivos retornam `ERR_FS_MV_FAILED`; o arquivo de origem permanece no lugar.

<SignatureBlock label="Assinatura">mv(src: string, dest: string): Result&lt;void, FsError&gt;</SignatureBlock>

A operação retorna `Err(FsError)` quando falha; inspecione com `match` ou propague com `?`.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/mv/basic-usage.md -->
