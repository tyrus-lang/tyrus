---
title: search
description: Busca arquivos que correspondem a um padrão.
---

# search

Busca arquivos que correspondem a um padrão glob, opcionalmente de forma recursiva.

<SignatureBlock label="Assinatura">search(pattern: string, options?: SearchOptions): Result&lt;string[], FsError&gt;</SignatureBlock>

## Opções

`SearchOptions` é uma interface com dois campos opcionais:

- `recursive?: boolean`: quando `true`, busca recursivamente nos subdiretórios. O padrão é `false`.
- `maxResults?: u32`: número máximo de resultados retornados. Por padrão, não há limite.

## Tratamento de erros

Retorna `Err(FsError)` com o código `ERR_FS_SEARCH_FAILED` se um diretório não puder ser listado ou uma entrada não puder ser inspecionada. Uma busca com falha não retorna resultados parciais; quando não há correspondências, o resultado é `Ok([])`.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/search/example.md -->
