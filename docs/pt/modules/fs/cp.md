---
title: cp
description: Copia um arquivo ou diretório.
---

# cp

Copia um arquivo ou diretório de `src` para `dest`. Para diretórios, use `{ recursive: true }`.

<SignatureBlock label="Assinatura">cp(src: string, dest: string, options?: CpOptions): Result&lt;void, FsError&gt;</SignatureBlock>

A operação retorna `Err(FsError)` quando falha; inspecione com `match` ou propague com `?`.

## Opções

`CpOptions` é uma interface com dois campos opcionais:

- `recursive?: boolean`: quando `true`, copia diretórios recursivamente. É obrigatório para origens que sejam diretórios. O padrão é `false`.
- `force?: boolean`: quando `true`, sobrescreve arquivos existentes no destino. O padrão é `false`.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/cp/basic-usage.md -->
