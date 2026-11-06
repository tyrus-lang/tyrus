---
title: diskUsage
description: Obtém informações sobre o uso do disco.
---

# diskUsage

Retorna informações de uso do disco do sistema de arquivos que contém o caminho informado.
No Unix, `free` é o espaço disponível para este processo conforme `statvfs`; no Windows, é o espaço disponível para o chamador conforme `GetDiskFreeSpaceExW`. `used` representa o espaço usado em todo o sistema de arquivos, então `used + free` pode ser menor que `total`.

<SignatureBlock label="Assinatura">diskUsage(path: string): Result&lt;DiskUsage, FsError&gt;</SignatureBlock>

## Campos

| Campo | Tipo | Descrição |
|-------|------|-------------|
| `total` | `u64` | Espaço total do disco em bytes. |
| `used` | `u64` | Espaço usado do disco em bytes. |
| `free` | `u64` | Bytes que este processo ainda pode gravar. O espaço reservado ao superusuário não entra, então `used + free` pode ser menor que `total`. |

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/disk-usage/example.md -->
