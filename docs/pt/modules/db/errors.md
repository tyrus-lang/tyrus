---
title: DbError
description: Consulte erros recuperáveis das operações de tyrus:db.
---

# DbError

`DbError` é o tipo estruturado de erro para falhas recuperáveis de banco de dados.

<SignatureBlock label="Assinatura">interface DbError { readonly code: string; readonly message: string }</SignatureBlock>

## Campos

| Campo | Tipo | Finalidade |
| --- | --- | --- |
| `code` | `string` | Identificador estável para tratamento programático, como `ERR_DB_QUERY_FAILED`. |
| `message` | `string` | Explicação legível da falha. |

Operações de banco retornam `Promise<Result<T, DbError>>`. Aguarde cada chamada para receber o `Result` e então trate falhas com `onfail`, `?` ou pattern matching. A API `tyrus:db` não usa `throws`.

## Códigos em runtime

| Código | Significado |
| --- | --- |
| `ERR_DB_CONNECTION_FAILED` | Não foi possível conectar ao backend de banco configurado. |
| `ERR_DB_CONNECTION_TIMEOUT` | A tentativa de conexão PostgreSQL, MySQL ou MongoDB excedeu `connectTimeoutMs`. |
| `ERR_DB_POOL_TIMEOUT` | O pool PostgreSQL ou MySQL não forneceu uma conexão, ou o runtime MongoDB não liberou uma vaga de operação, antes de `acquireTimeoutMs`. |
| `ERR_DB_QUERY_TIMEOUT` | Uma operação PostgreSQL, MySQL ou MongoDB excedeu `queryTimeoutMs`. |
| `ERR_DB_BUSY` | A fila limitada do worker de banco já contém 128 solicitações pendentes. |
| `ERR_DB_CONNECTION_CONFIGURATION` | A variável de ambiente indicada por `database.connectionEnv` está ausente. |
| `ERR_DB_INVALID_CONFIGURATION` | O destino de banco em runtime está malformado ou não corresponde a uma configuração suportada. |
| `ERR_DB_INVALID_CONNECTION` | A variável de conexão não contém uma URL ou URI válida para o backend configurado. O valor da conexão nunca é incluído na mensagem de erro. |
| `ERR_DB_TLS_CONFIGURATION` | Um arquivo TLS configurado para o banco conflita com a URL de conexão ou não permite verificar o certificado e o hostname. |
| `ERR_DB_BACKEND_UNAVAILABLE` | O backend configurado ainda não tem driver de runtime nesta build. |
| `ERR_DB_INVALID_SQL` | Não foi possível preparar o SQL para o backend configurado. |
| `ERR_DB_FEATURE_UNSUPPORTED` | O backend configurado não oferece suporte ao recurso de banco solicitado. |
| `ERR_DB_SQLITE_BUSY` | SQLite não obteve o bloqueio necessário antes do timeout. |
| `ERR_DB_SQLITE_CONSTRAINT` | Uma restrição do banco recusou a alteração. |
| `ERR_DB_EXECUTE_FAILED` | O backend de banco configurado recusou ou não conseguiu executar o comando. |
| `ERR_DB_INVALID_PARAMETERS` | Os parâmetros não puderam ser codificados como valores suportados. |
| `ERR_DB_PARAMETER_LIMIT` | Uma instrução tentou vincular mais de 999 parâmetros. |
| `ERR_DB_DECODE_FAILED` | O banco retornou um valor que não pôde ser decodificado para um tipo suportado. |
| `ERR_DB_VALUE_OUT_OF_RANGE` | Um valor não pôde ser representado pelo tipo de banco de destino. |
| `ERR_DB_DOCUMENT_TOO_LARGE` | Um documento BSON ou filtro MongoDB serializado excede 16 MiB. |
| `ERR_DB_VALUE_TOO_DEEP` | Um valor estruturado MongoDB excede 100 níveis de aninhamento. |
| `ERR_DB_INVALID_TEMPORAL_VALUE` | Um `Instant` MongoDB tem campos `seconds` ou `nanoseconds` inválidos. |
| `ERR_DB_INVALID_MODEL_VALUE` | Um valor de Model do MongoDB omite um campo obrigatório ou não corresponde ao tipo declarado. |
