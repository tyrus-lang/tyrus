---
title: metadata
description: Obtém metadados detalhados de arquivos.
---

# metadata

Retorna metadados detalhados de um arquivo ou diretório. Quando `path` é um link simbólico, os campos de metadados descrevem o destino e `isSymlink` indica que o caminho original é um link. Retorna `Err(FsError)` em caso de falha.

<SignatureBlock label="Assinatura">metadata(path: string): Result&lt;Metadata, FsError&gt;</SignatureBlock>

## Campos

| Campo | Tipo | Descrição |
|-------|------|-------------|
| `size` | `u64` | Tamanho do arquivo em bytes. |
| `isFile` | `boolean` | `true` se o caminho for um arquivo. |
| `isDirectory` | `boolean` | `true` se o caminho for um diretório. |
| `isSymlink` | `boolean` | `true` se o caminho for um link simbólico. |
| `modified` | `u64` | Horário da última modificação (segundos desde a época Unix). |
| `created` | `u64` | Horário de criação (segundos desde a época Unix). |
| `accessed` | `u64` | Horário do último acesso (segundos desde a época Unix). |
| `permissions` | `u32` | Bits de permissões do arquivo. |
| `uid` | `u32` | ID de usuário do proprietário do arquivo. |
| `gid` | `u32` | ID do grupo do proprietário do arquivo. |
| `mode` | `u32` | Tipo de arquivo e modo de permissões. |

Em plataformas que não são Unix, `permissions`, `uid`, `gid` e `mode` são atualmente `0`. Esses campos expõem bits de permissão POSIX e IDs de proprietário Unix; as ACLs do Windows não são convertidas para essa representação.

## Tratamento de erros

<!-- @include: ../../../.fragments/modules/fs/metadata/error-handling.md -->
