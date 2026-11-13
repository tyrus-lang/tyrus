---
title: mime
description: Obtém o tipo MIME pela extensão do arquivo.
---

# mime

Retorna o tipo MIME de um arquivo com base na extensão. Não retorna valor quando a extensão é desconhecida ou o caminho não tem extensão. Diferentemente das operações que retornam `Result`, `mime` não possui falha recuperável.

<SignatureBlock label="Assinatura">mime(path: string): string?</SignatureBlock>

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/mime/example.md -->
