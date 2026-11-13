---
title: glob
description: Encontra arquivos que correspondem a um padrão glob.
---

# glob

Compara nomes de arquivos e diretórios com um padrão glob. O padrão é resolvido em relação ao diretório de trabalho do processo antes da comparação. Padrões relativos retornam caminhos relativos, enquanto padrões absolutos retornam caminhos absolutos. Aceita `*` (corresponde a quaisquer caracteres, exceto `/`) e `**` (corresponde a qualquer quantidade de diretórios).

<SignatureBlock label="Assinatura">glob(pattern: string): Result&lt;string[], FsError&gt;</SignatureBlock>

## Tratamento de erros

Retorna `Err(FsError)` com o código `ERR_FS_GLOB_FAILED` se um diretório não puder ser listado ou uma entrada necessária à travessia não puder ser inspecionada. Uma travessia com falha não retorna correspondências parciais; quando não há correspondências, o resultado é `Ok([])`.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/glob/example.md -->
