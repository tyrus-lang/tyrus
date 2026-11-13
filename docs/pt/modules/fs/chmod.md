---
title: chmod
description: Altera as permissões de um arquivo.
---

# chmod

Altera as permissões de um arquivo ou diretório. O parâmetro `mode` recebe um valor do enum `FileMode`. Em plataformas que não são Unix, retorna `Err` com o código `ERR_FS_CHMOD_FAILED`, pois os bits de permissão POSIX não são compatíveis.

<SignatureBlock label="Assinatura">chmod(path: string, mode: FileMode): Result&lt;void, FsError&gt;</SignatureBlock>

## Membros de FileMode

| Membro | Valor | Octal |
|--------|-------|-------|
| `Read` | 256 | 0o400 |
| `Write` | 128 | 0o200 |
| `Execute` | 64 | 0o100 |
| `OwnerRead` | 256 | 0o400 |
| `OwnerWrite` | 128 | 0o200 |
| `OwnerExecute` | 64 | 0o100 |
| `GroupRead` | 32 | 0o040 |
| `GroupWrite` | 16 | 0o020 |
| `GroupExecute` | 8 | 0o010 |
| `OtherRead` | 4 | 0o004 |
| `OtherWrite` | 2 | 0o002 |
| `OtherExecute` | 1 | 0o001 |
| `AllRead` | 292 | 0o444 |
| `AllWrite` | 146 | 0o222 |
| `AllExecute` | 73 | 0o111 |
| `FullOwner` | 448 | 0o700 |
| `FullGroup` | 56 | 0o070 |
| `FullOther` | 7 | 0o007 |
| `Full` | 511 | 0o777 |

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/chmod/example.md -->
