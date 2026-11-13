---
title: Tipos do tyrus:archive
description: Tipos de entrada, resumo, opções e erro exportados pelo tyrus:archive.
---

# Tipos do tyrus:archive

::: tip Tipos precisam ser importados pelo nome
`ArchiveEntry`, `ArchiveEntryKind`, `ArchiveError`, `ArchiveFormat`, `ArchiveSummary`, `PackOptions` e `UnpackOptions` precisam ser importados explicitamente sempre que seus campos, membros ou literais de objeto forem usados.
:::

## ArchiveSummary

`ArchiveSummary` é retornado por [`pack`](./pack.md) e [`unpack`](./unpack.md).

| Campo | Tipo | Descrição |
|-------|------|-------------|
| `entries` | `u64` | Arquivos e diretórios gravados. |
| `bytes` | `u64` | Bytes de arquivo descomprimidos gravados. |

## ArchiveEntry

`ArchiveEntry` é retornado por [`list`](./list.md).

| Campo | Tipo | Descrição |
|-------|------|-------------|
| `name` | `string` | O nome da entrada exatamente como o arquivo o guarda. Nomes de diretório terminam com `/` em arquivos ZIP. |
| `kind` | `ArchiveEntryKind` | O que a entrada é. |
| `size` | `u64` | Tamanho descomprimido em bytes. |
| `compressedSize` | `u64?` | Tamanho comprimido em bytes para entradas ZIP; ausente para entradas TAR. |

## ArchiveEntryKind

| Membro | Significado |
|--------|---------|
| `File` | Um arquivo regular. |
| `Directory` | Um diretório. |
| `Symlink` | Um link simbólico. O `unpack` o recusa. |
| `HardLink` | Um link físico. O `unpack` o recusa. |
| `Other` | Um dispositivo, pipe ou outra entrada especial. O `unpack` a recusa. |

<!-- @include: ../../../.fragments/modules/archive/types/entry-kind.md -->

## ArchiveFormat

O formato que [`packStream`](./pack-stream.md) escreve, já que uma stream não tem extensão para decidi-lo.

| Membro | Formato |
|--------|--------|
| `Zip` | ZIP, com data descriptors. |
| `Tar` | TAR, sem compressão. |
| `TarGz` | TAR comprimido com Gzip. |

## PackOptions

| Campo | Tipo | Padrão |
|-------|------|---------|
| `level` | `i32?` | `6` |
| `overwrite` | `boolean?` | `false` |
| `exclude` | `string[]?` | nenhum |
| `prefix` | `string?` | nenhum |
| `reproducible` | `boolean?` | `false` |
| `timeoutMs` | `u64?` | `600000` |

Veja em [`pack`](./pack.md#todas-as-opcoes) o que cada opção faz.

## UnpackOptions

| Campo | Tipo | Padrão |
|-------|------|---------|
| `overwrite` | `boolean?` | `false` |
| `stripComponents` | `u32?` | `0` |
| `maxEntries` | `u64?` | `100000` |
| `maxBytes` | `u64?` | `4 GiB` |
| `maxRatio` | `u32?` | `200` |
| `timeoutMs` | `u64?` | `600000` |

Veja em [`unpack`](./unpack.md#todas-as-opcoes) o que cada opção faz.

## ArchiveError

`ArchiveError` é o erro recuperável de toda operação de arquivo, entregue como o `Err` do `Result` com que cada função resolve.

| Campo | Tipo | Descrição |
|-------|------|-------------|
| `code` | `string` | Identificador estável, como `ERR_ARCHIVE_PATH_ESCAPE`. Veja os [códigos de erro](./index.md#codigos-de-erro). |
| `message` | `string` | Descrição legível. |
| `entry` | `string?` | A entrada do arquivo ou o caminho de origem que causou a falha, quando houver, truncado em 4 KiB. |

<!-- @include: ../../../.fragments/modules/archive/types/error.md -->
