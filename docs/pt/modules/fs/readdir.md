---
title: readdir
description: Lê o conteúdo de um diretório.
---

# readdir

Lê o conteúdo de um diretório e retorna um array de objetos `DirEntry`.

<SignatureBlock label="Assinatura">readdir(path: string, options?: ReaddirOptions): Result&lt;DirEntry[], FsError&gt;</SignatureBlock>

## Opções de leitura

`ReaddirOptions` é uma interface com um campo opcional:

- `recursive?: boolean`: quando `true`, percorre subdiretórios recursivamente e inclui suas entradas no resultado. O padrão é `false`.

## Entradas de diretório

Cada `DirEntry` tem os campos somente para leitura `name: string` e `type: DirEntryType`. `DirEntryType` é um enum com os membros `File`, `Directory`, `Symlink` e `Other`, permitindo identificar o tipo da entrada sem inspecionar seu nome.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/readdir/basic-usage.md -->

## Tratamento de erros

Retorna `Err(FsError)` com o código `ERR_FS_READ_FAILED` se o diretório raiz ou qualquer diretório percorrido recursivamente não puder ser lido, ou se uma entrada não puder ser inspecionada. A operação não retorna uma listagem parcial.

::: danger error
<!-- @include: ../../../.fragments/modules/fs/readdir/error-handling.md -->
:::
