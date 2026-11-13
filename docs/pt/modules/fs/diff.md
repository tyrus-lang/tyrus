---
title: diff
description: Compara dois arquivos ou diretórios.
---

# diff

Compara dois arquivos ou diretórios e retorna um `DiffResult` que descreve as diferenças.

<SignatureBlock label="Assinatura">diff(path1: string, path2: string): Result&lt;DiffResult, FsError&gt;</SignatureBlock>

## Campos de DiffResult

| Campo | Tipo | Descrição |
|-------|------|-------------|
| `added` | `string[]` | Arquivos presentes em path1, mas não em path2. |
| `removed` | `string[]` | Arquivos presentes em path2, mas não em path1. |
| `modified` | `string[]` | Arquivos presentes em ambos, mas com conteúdo diferente. |
| `unchanged` | `string[]` | Arquivos presentes em ambos com conteúdo idêntico. |

## Tratamento de erros

Retorna `Err(FsError)` com o código `ERR_FS_DIFF_FAILED` se algum caminho não puder ser inspecionado, um diretório não puder ser listado ou um arquivo correspondente não puder ser lido. A operação não informa falhas de leitura como diferenças.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/diff/example.md -->
