---
title: tempDir
description: Retorna o diretório temporário do sistema.
---

# tempDir

Retorna uma struct `TempDir` com o campo `path`, que contém o caminho do diretório temporário do sistema.

<SignatureBlock label="Assinatura">tempDir(): TempDir</SignatureBlock>

## Campos de TempDir

| Campo | Tipo | Descrição |
|-------|------|-------------|
| `path` | `string` | Caminho do diretório temporário do sistema. |

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/temp-dir/example.md -->
