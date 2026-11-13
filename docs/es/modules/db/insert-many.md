---
title: insertMany
description: Inserta un lote limitado de registros del modelo.
---

# Model.insertMany

Llama a `User.insertMany(rows)` en un modelo importado para insertar varios registros en una sola operación del backend. Todas las filas deben proporcionar los mismos campos en el mismo orden. Los backends SQL enlazan los valores como parámetros, y un error de constraint rechaza la operación con un `DbError` recuperable.

El tipo de retorno es `Promise<Result<User[], DbError>>`; espera la llamada antes de usar el `Result`. Si tiene éxito, los registros insertados se devuelven como `User[]`, incluidos los identificadores y timestamps generados. El lote debe contener de 1 a 500 documentos. SQLite, PostgreSQL y MySQL usan el límite conservador de Tyrus de 999 parámetros SQL enlazados, por lo que el límite efectivo de filas es `min(500, floor(999 / cantidadDeCampos))`; por ejemplo, las filas con seis campos proporcionados permiten como máximo 166 registros por llamada. MongoDB envía documentos BSON en lugar de parámetros SQL, así que este límite no se aplica; admite hasta 500 documentos, sujeto al límite BSON de 16 MiB por documento.

Las columnas obligatorias, no nullable y sin default en la base de datos deben proporcionarse en todas las filas. Se pueden omitir las columnas nullable, las columnas con default en el snapshot local de la base de datos y la única clave primaria `@id` del modelo. La clave primaria omitida solo se genera cuando el schema de la tabla configurada define una identidad o un valor default.

Los backends SQL ejecutan el lote como una sola sentencia. MongoDB valida todos los documentos antes de enviar el lote y usa una operación de inserción ordenada, pero un fallo del servidor puede dejar documentos insertados antes del error; `insertMany` no promete rollback entre backends. Si tiene éxito, devuelve todos los recursos insertados en el orden de entrada, incluidos los identificadores generados. MongoDB todavía puede devolver un error recuperable después de un progreso parcial en el servidor.

Este fragmento supone un modelo `User` importado con un campo `email`.

<!-- @include: ../../../.fragments/modules/db/insert-many/example.md -->

Usa [`Model.insertOne(data)`](./insert-one.md) para insertar un único registro.
