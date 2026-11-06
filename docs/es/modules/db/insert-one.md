---
title: insertOne
description: Inserta un registro del modelo con valores enlazados.
---

# Model.insertOne

Llama a `User.insertOne(data)` en un modelo importado para insertar un registro. Las claves del objeto son nombres de campos del modelo; el compilador las mapea a las columnas declaradas en la base de datos y enlaza cada valor como parámetro.

El tipo de retorno es `Promise<Result<User, DbError>>`; espera la llamada antes de usar el `Result`. Si tiene éxito, se devuelve el `User` insertado con su ID y timestamps generados. Los errores de constraints o conexión con la base de datos se devuelven como `Err(error)`.

Las columnas obligatorias, no nullable y sin default en la base de datos deben proporcionarse. Se pueden omitir las columnas nullable, las columnas con default en el snapshot local de la base de datos y la única clave primaria `@id` del modelo. La clave primaria omitida solo se genera cuando el schema de la tabla configurada define una identidad o un valor default.

Este fragmento supone un modelo `User` importado con un campo `email`.

<!-- @include: ../../../.fragments/modules/db/insert-one/example.md -->

Usa [`Model.insertMany(rows)`](./insert-many.md) para insertar un lote en una sola instrucción.
