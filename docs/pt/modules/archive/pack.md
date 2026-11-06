---
title: pack
description: Cria um arquivo ZIP, TAR ou TAR.GZ a partir de um arquivo ou diretório.
---

# pack

`pack` cria um arquivo a partir de um arquivo ou de um diretório. A extensão do destino, em minúsculas, decide o formato: `.zip`, `.tar`, `.tar.gz` ou `.tgz`.

As entradas são gravadas em ordem estável com separadores `/`, e o arquivo é gravado primeiro num arquivo temporário ao lado do destino e depois renomeado para o nome final, então uma falha nunca deixa um arquivo truncado. Quando o destino fica dentro do diretório de origem, ele fica de fora do arquivo.

<SignatureBlock label="Assinatura">pack(source: string, destination: string, options?: PackOptions): Promise&lt;Result&lt;ArchiveSummary, ArchiveError&gt;&gt;</SignatureBlock>

## Exemplo mínimo

<!-- @include: ../../../.fragments/modules/archive/pack/minimal.md -->

## Todas as opções

<!-- @include: ../../../.fragments/modules/archive/pack/maximal.md -->

| Opção | Tipo | Padrão | Efeito |
|--------|------|---------|--------|
| `level` | `i32` | `6` | Nível de compressão de `0` (só armazena) a `9` (menor saída). Não é aceito para `.tar`. |
| `overwrite` | `boolean` | `false` | Substitui um arquivo de destino existente. Um diretório nunca é substituído. |
| `exclude` | `string[]` | nenhum | Padrões glob, relativos à origem, de arquivos e diretórios a deixar de fora. `**` abrange qualquer quantidade de diretórios. |
| `prefix` | `string` | nenhum | Diretório que contém todas as entradas, como `release-1.0.0`. |
| `reproducible` | `boolean` | `false` | Grava datas fixas e permissões normalizadas, para que a mesma entrada produza os mesmos bytes. |
| `timeoutMs` | `u64` | `600000` (10 minutos) | Desiste com `ERR_ARCHIVE_TIMEOUT` quando a criação demora mais, sem deixar arquivo parcial. `0` falha com `ERR_ARCHIVE_INVALID_OPTION`. |

Links simbólicos e arquivos especiais na origem são recusados com `ERR_ARCHIVE_UNSUPPORTED_ENTRY`, que informa o caminho em `entry`, em vez de serem ignorados em silêncio.

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/archive/pack/error-handling.md -->
:::

Um destino literal com extensão desconhecida ou em maiúsculas, e um `level` literal que o formato não aceita, são apontados pelo compilador antes de o programa rodar.
