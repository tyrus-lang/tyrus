---
title: DbError
description: Consulta errores recuperables de las operaciones de tyrus:db.
---

# DbError

`DbError` es el tipo estructurado de error para fallos recuperables de base de datos.

<SignatureBlock label="Firma">interface DbError { readonly code: string; readonly message: string }</SignatureBlock>

## Campos

| Campo | Tipo | Finalidad |
| --- | --- | --- |
| `code` | `string` | Identificador estable para el tratamiento programático, como `ERR_DB_QUERY_FAILED`. |
| `message` | `string` | Explicación legible del fallo. |

Las operaciones de base devuelven `Promise<Result<T, DbError>>`. Espera cada llamada para recibir el `Result` y luego trata los fallos con `onfail`, `?` o pattern matching. La API `tyrus:db` no usa `throws`.

## Códigos en runtime

| Código | Significado |
| --- | --- |
| `ERR_DB_CONNECTION_FAILED` | No se pudo conectar al backend de base de datos configurado. |
| `ERR_DB_CONNECTION_TIMEOUT` | El intento de conexión a PostgreSQL, MySQL o MongoDB superó `connectTimeoutMs`. |
| `ERR_DB_POOL_TIMEOUT` | El pool de PostgreSQL o MySQL no pudo proporcionar una conexión, o el runtime de MongoDB no liberó un cupo de operación, antes de `acquireTimeoutMs`. |
| `ERR_DB_QUERY_TIMEOUT` | Una operación de PostgreSQL, MySQL o MongoDB superó `queryTimeoutMs`. |
| `ERR_DB_BUSY` | La cola limitada del worker de base de datos ya contiene 128 solicitudes pendientes. |
| `ERR_DB_CONNECTION_CONFIGURATION` | Falta la variable de entorno indicada por `database.connectionEnv`. |
| `ERR_DB_INVALID_CONFIGURATION` | El destino de base de datos en runtime está mal formado o no coincide con una configuración compatible. |
| `ERR_DB_INVALID_CONNECTION` | La variable de conexión no contiene una URL o URI válida para el backend configurado. El valor de conexión nunca se incluye en el mensaje de error. |
| `ERR_DB_TLS_CONFIGURATION` | Un archivo TLS configurado para la base entra en conflicto con la URI de conexión o no permite verificar el certificado y el hostname. |
| `ERR_DB_BACKEND_UNAVAILABLE` | El backend configurado todavía no tiene un controlador de runtime en esta compilación. |
| `ERR_DB_INVALID_SQL` | No se pudo preparar el SQL para el backend configurado. |
| `ERR_DB_FEATURE_UNSUPPORTED` | El backend configurado no admite la funcionalidad de base de datos solicitada. |
| `ERR_DB_SQLITE_BUSY` | SQLite no consiguió el bloqueo necesario antes del timeout. |
| `ERR_DB_SQLITE_CONSTRAINT` | Una restricción de la base rechazó el cambio. |
| `ERR_DB_EXECUTE_FAILED` | El backend de base de datos configurado rechazó o no pudo ejecutar el comando. |
| `ERR_DB_INVALID_PARAMETERS` | Los parámetros no se pudieron codificar como valores admitidos. |
| `ERR_DB_PARAMETER_LIMIT` | Una instrucción intentó vincular más de 999 parámetros. |
| `ERR_DB_DECODE_FAILED` | La base de datos devolvió un valor que no se pudo decodificar en un tipo admitido. |
| `ERR_DB_VALUE_OUT_OF_RANGE` | Un valor no se pudo representar con el tipo de base de datos de destino. |
| `ERR_DB_DOCUMENT_TOO_LARGE` | Un documento BSON o filtro MongoDB serializado supera los 16 MiB. |
| `ERR_DB_VALUE_TOO_DEEP` | Un valor estructurado de MongoDB supera los 100 niveles de anidamiento. |
| `ERR_DB_INVALID_TEMPORAL_VALUE` | Un `Instant` de MongoDB tiene campos `seconds` o `nanoseconds` no válidos. |
| `ERR_DB_INVALID_MODEL_VALUE` | Un valor de Model de MongoDB omite un campo obligatorio o no coincide con el tipo declarado. |
