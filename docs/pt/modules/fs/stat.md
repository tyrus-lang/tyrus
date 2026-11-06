---
title: stat
description: Obtém estatísticas de um arquivo.
---

# stat

Retorna estatísticas de um arquivo ou diretório. Caminhos inválidos retornam `Err` com `ERR_FS_INVALID_PATH`; caminhos que não podem ser inspecionados retornam `Err` com `ERR_FS_STAT_FAILED`.

Quando `path` é um link simbólico, as estatísticas descrevem o destino e `isSymlink` é `true` para o caminho original.

<SignatureBlock label="Assinatura">stat(path: string): Result&lt;FileStats, FsError&gt;</SignatureBlock>

## Campos

| Campo | Tipo | Descrição |
|-------|------|-------------|
| `size` | `u64` | Tamanho do arquivo em bytes. |
| `isFile` | `boolean` | `true` se o caminho for um arquivo. |
| `isDirectory` | `boolean` | `true` se o caminho for um diretório. |
| `isSymlink` | `boolean` | `true` se o caminho for um link simbólico. |
| `modified` | `u64` | Horário da última modificação (segundos desde a época Unix). |
| `created` | `u64` | Horário de criação (segundos desde a época Unix). |
| `permissions` | `u32` | Bits de permissões do arquivo. |

Para um caminho acessível em plataformas que não são Unix, `permissions` é `0`; esta API expõe bits de permissão POSIX e não converte ACLs do Windows.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/stat/example.md -->
