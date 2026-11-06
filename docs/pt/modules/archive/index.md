---
title: tyrus:archive
description: Cria, inspeciona e extrai com segurança arquivos ZIP, TAR e TAR.GZ.
---

# tyrus:archive

`tyrus:archive` cria, inspeciona e extrai arquivos ZIP, TAR e TAR.GZ a partir de caminhos em disco. Toda operação é assíncrona, transfere os dados por buffers limitados e nunca carrega um arquivo inteiro na memória; `packStream` e `unpackStream` fazem o mesmo com uma stream de bytes no lugar do arquivo.

A extração é segura por padrão: nenhuma entrada pode ser gravada fora do diretório de destino, links e arquivos especiais são recusados, limites de quantidade de entradas, tamanho extraído e razão de compressão barram bombas de descompressão, e toda operação desiste depois de um limite de tempo.

## Imports

<SignatureBlock label="Importações">import { ArchiveEntry, ArchiveEntryKind, ArchiveError, ArchiveFormat, ArchiveSummary, PackOptions, UnpackOptions, list, pack, packStream, unpack, unpackStream } from "tyrus:archive";</SignatureBlock>

## Funções

| Item | Assinatura | Finalidade |
|------|-----------|---------|
| [`pack`](./pack.md) | `(source: string, destination: string, options?: PackOptions) => Promise<Result<ArchiveSummary, ArchiveError>>` | Cria um arquivo a partir de um arquivo ou diretório. |
| [`list`](./list.md) | `(archive: string, options?: UnpackOptions) => Promise<Result<ArchiveEntry[], ArchiveError>>` | Lê as entradas de um arquivo sem extraí-lo. |
| [`unpack`](./unpack.md) | `(archive: string, destination: string, options?: UnpackOptions) => Promise<Result<ArchiveSummary, ArchiveError>>` | Extrai um arquivo para um diretório. |
| [`packStream`](./pack-stream.md) | `(source: string, format: ArchiveFormat, options?: PackOptions) => ReadableStream<u8[]>` | Escreve o arquivo de um arquivo ou diretório numa stream de bytes. |
| [`unpackStream`](./unpack-stream.md) | `(archive: ReadableStream<u8[]>, destination: string, options?: UnpackOptions) => Promise<Result<ArchiveSummary, ArchiveError>>` | Extrai um arquivo TAR ou TAR.GZ de uma stream de bytes. |

## Formatos

`pack` decide o formato pela extensão do destino, que deve estar em minúsculas. `list` e `unpack` ignoram o nome do arquivo e identificam o formato pelo conteúdo, então um `.jar`, `.epub` ou `.docx`, que são arquivos ZIP, é extraído como qualquer `.zip`.

| Extensão | Formato | Compressão |
|-----------|--------|-------------|
| `.zip` | ZIP | Deflate, níveis `0` a `9` |
| `.tar` | TAR | Nenhuma |
| `.tar.gz`, `.tgz` | TAR.GZ | Gzip, níveis `0` a `9` |

## Códigos de erro

Toda função resolve com um `Result` e nunca rejeita: o sucesso é `Ok` com o valor, e uma falha é `Err` com um [`ArchiveError`](./types.md#archiveerror). Seu `code` é estável, seu `message` é legível e seu `entry` nomeia a entrada do arquivo que causou a falha, quando houver uma. Dentro de uma função que retorna um `Result` com o mesmo erro, `await pack(...)?` entrega o valor ou retorna o erro.

| Código | Significado |
|------|---------|
| `ERR_ARCHIVE_NOT_FOUND` | O arquivo, a origem ou o diretório pai do destino não existe. |
| `ERR_ARCHIVE_IO` | A leitura ou a escrita falhou. |
| `ERR_ARCHIVE_PERMISSION_DENIED` | O sistema operacional negou o acesso. |
| `ERR_ARCHIVE_UNSUPPORTED_FORMAT` | O conteúdo não é um arquivo ZIP, TAR ou TAR.GZ. |
| `ERR_ARCHIVE_UNSUPPORTED_EXTENSION` | O destino do `pack` não termina com `.zip`, `.tar`, `.tar.gz` ou `.tgz`. |
| `ERR_ARCHIVE_INVALID_ARCHIVE` | O arquivo está malformado ou truncado. |
| `ERR_ARCHIVE_CHECKSUM_MISMATCH` | Os dados de uma entrada não batem com o checksum. |
| `ERR_ARCHIVE_ENCRYPTED` | O arquivo contém uma entrada criptografada. |
| `ERR_ARCHIVE_INVALID_OPTION` | Uma opção tem valor inválido, como um limite `0`. |
| `ERR_ARCHIVE_INVALID_COMPRESSION_LEVEL` | `level` está fora de `0` a `9` ou foi informado para um destino `.tar`. |
| `ERR_ARCHIVE_DESTINATION_EXISTS` | O destino já existe e `overwrite` não foi ativado. |
| `ERR_ARCHIVE_PATH_ESCAPE` | Uma entrada seria gravada fora do diretório de destino. |
| `ERR_ARCHIVE_INVALID_ENTRY_NAME` | O nome de uma entrada está malformado ou não pode ser criado em todas as plataformas. |
| `ERR_ARCHIVE_DUPLICATE_ENTRY` | Duas entradas resultam no mesmo caminho. |
| `ERR_ARCHIVE_UNSAFE_LINK` | O arquivo contém um link simbólico ou físico. |
| `ERR_ARCHIVE_UNSUPPORTED_ENTRY` | O arquivo usa um tipo de entrada ou método de compressão não suportado, ou a origem contém um link ou arquivo especial. |
| `ERR_ARCHIVE_ENTRY_LIMIT` | O arquivo tem mais entradas que `maxEntries`. |
| `ERR_ARCHIVE_SIZE_LIMIT` | A extração gravaria mais bytes que `maxBytes`. |
| `ERR_ARCHIVE_RATIO_LIMIT` | Uma entrada, ou um arquivo TAR.GZ como um todo, expande mais que `maxRatio` vezes o tamanho comprimido. |
| `ERR_ARCHIVE_ALLOCATION_FAILED` | Não foi possível alocar memória para o resultado. |
| `ERR_ARCHIVE_NO_SPACE` | O disco ficou sem espaço, ou um arquivo ZIP declara mais conteúdo do que o destino tem livre. |
| `ERR_ARCHIVE_TIMEOUT` | A operação demorou mais que `timeoutMs`. |
| `ERR_ARCHIVE_ZIP_NOT_STREAMABLE` | Um arquivo ZIP foi passado a `unpackStream`, que não extrai ZIP de uma stream. |

## Exemplo

<!-- @include: ../../../.fragments/modules/archive/index/example.md -->
