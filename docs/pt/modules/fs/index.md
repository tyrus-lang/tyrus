---
title: tyrus:fs
description: APIs nativas de filesystem com suporte a async, respaldadas pelo runtime do tyrus.
---

# tyrus:fs

`tyrus:fs` expõe operações nativas de filesystem fornecidas pelo runtime nativo. Ele fornece variantes síncronas e assíncronas com `FsError` recuperável.

## Importações

<!-- @include: ../../../.fragments/modules/fs/imports.md -->

## Funções

| Função | Assinatura | Finalidade |
|--------|------------|------------|
| [`readFile`](./read-file.md) | `(path: string) => Promise<Result<string, FsError>>` | Lê um arquivo UTF-8 de forma assíncrona. |
| [`readFileStream`](./read-file-stream.md) | `(path: string) => Result<ReadableStream<u8[]>, FsError>` | Lê chunks binários limitados progressivamente. |
| [`readFileSync`](./read-file-sync.md) | `(path: string) => Result<string, FsError>` | Lê um arquivo UTF-8 de forma síncrona. |
| [`writeFile`](./write-file.md) | `(path: string, data: string) => Promise<Result<void, FsError>>` | Escreve uma string em um arquivo de forma assíncrona. |
| [`writeFileStream`](./write-file-stream.md) | `(path: string) => Result<WritableStream<u8[]>, FsError>` | Escreve chunks binários limitados progressivamente. |
| [`writeFileSync`](./write-file-sync.md) | `(path: string, data: string) => Result<void, FsError>` | Escreve uma string em um arquivo de forma síncrona. |
| [`exists`](./exists.md) | `(path: string) => boolean` | Verifica se um caminho existe. |
| [`appendFile`](./append-file.md) | `(path: string, data: string) => Promise<Result<void, FsError>>` | Acrescenta dados a um arquivo de forma assíncrona. |
| [`readdir`](./readdir.md) | `(path: string, options?: ReaddirOptions) => Result<DirEntry[], FsError>` | Lê entradas de um diretório. |
| [`mkdir`](./mkdir.md) | `(path: string) => Result<void, FsError>` | Cria um diretório. |
| [`rm`](./rm.md) | (path: string, options?: RmOptions) => Result<void, FsError> | Remove um arquivo ou diretório. |
| [`cp`](./cp.md) | (src: string, dest: string, options?: CpOptions) => Result<void, FsError> | Copia um arquivo ou diretório. |
| [`mv`](./mv.md) | (src: string, dest: string) => Result<void, FsError> | Move um arquivo ou diretório. |
| [`removeAll`](./remove-all.md) | (path: string) => Result<void, FsError> | Remove recursivamente um diretório. |
| [`stat`](./stat.md) | `(path: string) => Result<FileStats, FsError>` | Obtém estatísticas do arquivo ou um erro estruturado do filesystem. |
| [`metadata`](./metadata.md) | `(path: string) => Result<Metadata, FsError>` | Obtém metadados detalhados. |
| [`diskUsage`](./disk-usage.md) | `(path: string) => Result<DiskUsage, FsError>` | Obtém informações de uso de disco. |
| [`mime`](./mime.md) | `(path: string) => string?` | Obtém tipo MIME da extensão. |
| [`chmod`](./chmod.md) | `(path: string, mode: FileMode) => Result<void, FsError>` | Altera permissões do arquivo. |
| [`chown`](./chown.md) | `(path: string, uid: u32, gid: u32) => Result<void, FsError>` | Altera propriedade do arquivo. |
| [`readlink`](./readlink.md) | `(path: string) => Result<string, FsError>` | Lê o alvo de um link simbólico. |
| [`symlink`](./symlink.md) | `(original: string, link: string) => Result<void, FsError>` | Cria um link simbólico. |
| [`symlinkDir`](./symlink-dir.md) | `(original: string, link: string) => Result<void, FsError>` | Cria um link simbólico para diretório. |
| [`hardLink`](./hard-link.md) | `(original: string, link: string) => Result<void, FsError>` | Cria um link físico. |
| [`canonicalize`](./canonicalize.md) | `(path: string) => Result<string, FsError>` | Resolve para um caminho absoluto. |
| [`glob`](./glob.md) | `(pattern: string) => Result<string[], FsError>` | Encontra arquivos por padrão glob. |
| [`search`](./search.md) | `(pattern: string, options?: SearchOptions) => Result<string[], FsError>` | Busca arquivos por padrão. |
| [`diff`](./diff.md) | `(path1: string, path2: string) => Result<DiffResult, FsError>` | Compara dois arquivos ou diretórios. |
| [`atomicWrite`](./atomic-write.md) | `(path: string, data: string) => Result<void, FsError>` | Escreve atomicamente. |
| [`tempDir`](./temp-dir.md) | `() => TempDir` | Retorna o diretório temporário. |
| [`tempFile`](./temp-file.md) | `() => Result<string, FsError>` | Cria um arquivo temporário. |
| [`watch`](./watch.md) | `(path: string) => Result<FSWatcher, FsError>` | Começa a observar um arquivo ou diretório. |

::: warning Erros recuperáveis
Todas as operações recuperáveis do sistema de arquivos retornam `Result<T, FsError>`. Chamadas síncronas retornam `Result<T, FsError>` diretamente, enquanto operações assíncronas de arquivo retornam `Promise<Result<T, FsError>>`; construtores de stream retornam `Result<ReadableStream<u8[]>, FsError>` ou `Result<WritableStream<u8[]>, FsError>`. Use `match` para tratar o resultado ou propague-o com `?`. O erro possui um campo `code` estável (ex.: `ERR_FS_NOT_FOUND`) e uma `message` legível.
:::

## Caminhos multiplataforma

Todo caminho passado a qualquer função `fs` é resolvido de forma transparente antes de chegar ao sistema operacional, sem exigir nenhum import de módulo `path`:

- **Separadores**: uma contrabarra escrita no estilo Windows (`"data\\file.txt"`) é normalizada para barra normal no Linux/macOS. O próprio Windows já aceita nativamente as duas formas, então nada muda lá.
- **Segmentos relativos**: `./` e `../` são resolvidos contra o diretório de trabalho atual, `sub/../file.txt` e `file.txt` apontam para o mesmo lugar, e nenhum dos dois exige que `sub` de fato exista no disco.

Esta é uma resolução **lexical**, uma resolução lexical de caminhos, não uma canonicalização de sistema de arquivos: ela nunca toca o sistema de arquivos e nunca exige que o caminho já exista. Exigir existência quebraria toda chamada que cria um arquivo novo, que é o caso comum de `writeFileSync`.

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/example.md -->
