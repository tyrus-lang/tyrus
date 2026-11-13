---
title: atomicWrite
description: Grava um arquivo de forma atômica.
---

# atomicWrite

Grava dados em um arquivo de forma atômica: primeiro em um arquivo temporário exclusivo no diretório de destino e depois substitui o destino. Isso evita gravações parciais em caso de falha e funciona quando o arquivo já existe no Windows, macOS e Linux.

<SignatureBlock label="Assinatura">atomicWrite(path: string, data: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/atomic-write/example.md -->
