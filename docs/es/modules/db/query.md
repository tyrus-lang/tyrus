---
title: query
description: Ejecuta consultas relacionales tipadas y mutaciones parametrizadas.
---

# db.query

<SignatureBlock label="Firma">query&lt;T&gt;(sql: string, parameters: unknown[]): Promise&lt;Result&lt;T[], DbError&gt;&gt;</SignatureBlock>

<SignatureBlock label="Firma">query(sql: string, parameters: unknown[]): Promise&lt;Result&lt;void, DbError&gt;&gt;</SignatureBlock>

`db.query<T>` devuelve `Promise<Result<T[], DbError>>` para instrucciones `SELECT` validadas y decodifica cada fila devuelta en la interfaz `T`, que debe indicarse explícitamente. Espera cada llamada a la base; una consulta sin filas coincidentes se resuelve como un array vacío dentro de `Ok`. Para `INSERT`, `UPDATE` y `DELETE`, llama a `db.query` sin argumento de tipo; una mutación correcta se resuelve como `void` dentro de `Ok`.

Un `Instant` enviado como parámetro SQL se enlaza usando la representación del backend seleccionado. SQLite almacena texto UTC, PostgreSQL usa `TIMESTAMPTZ` y rechaza valores con precisión inferior a microsegundos, y MySQL usa `DATETIME(3)` y rechaza valores con precisión inferior a milisegundos. Un `SELECT *` tipado decodifica los campos `Instant` del Model a la representación de Tyrus.

`COUNT`, `SUM`, `AVG`, `MIN` y `MAX` se admiten en `SELECT`. Consulta [Agregaciones SQL](./aggregates.md) para los tipos de resultado, los alias, ejemplos y el comportamiento sin coincidencias.

Dentro de una cadena de `db.query`, el Language Server completa expresiones de agregación, cláusulas SQL, tablas y columnas del snapshot local del schema. Pasa el cursor sobre una tabla o columna para ver el schema, o sobre el nombre de una agregación para ver su firma y el comportamiento del resultado.

Para inspeccionar una consulta en el editor, coloca el cursor dentro de la cadena SQL, abre las Acciones de código y elige **Explain SQL query**. El resultado incluye el SQL normalizado, los tipos de parámetros con los valores ocultos y los campos devueltos; la validación usa el snapshot local sin abrir una conexión a la base de datos. La explicación en el editor acepta cadenas SQL de hasta 64 KiB.

Los joins internos y externos requieren que los campos de resultado reflejen el lado que puede faltar. Consulta [Joins SQL](./joins.md).

<!-- @include: ../../../.fragments/modules/db/query/example.md -->

El compilador valida el tipo de instrucción, las columnas seleccionadas, los alias, la cantidad y los tipos de los parámetros con el snapshot local del schema. Rechaza arrays con más de 999 parámetros mediante `E3305`; el runtime aplica el mismo límite y devuelve `ERR_DB_PARAMETER_LIMIT` como protección final. El SQL debe ser un literal de cadena y los valores deben proporcionarse en un literal de array separado. Consulta [Seguridad SQL](./security.md) para el subconjunto de instrucciones admitido y las reglas de vinculación.
