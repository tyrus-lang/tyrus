---
title: updateMany
description: Actualiza todos los registros del modelo que coinciden con un filtro no vacío.
---

# Model.updateMany

Llama a `User.updateMany(filter, changes)` en un modelo importado. El filtro debe ser un objeto no vacío y `changes` debe contener al menos un campo conocido del modelo. El campo `@id` no se puede modificar.

La actualización está parametrizada y se aplica a todos los registros que coinciden con el filtro. Devuelve `Promise<Result<u64, DbError>>`; el valor de éxito es la cantidad de registros actualizados. Los errores de constraints o conexión con la base de datos son valores `DbError` recuperables.

Este fragmento supone un modelo `User` importado con `id` marcado con `@id` y los campos `email` y `displayName`.

<!-- @include: ../../../.fragments/modules/db/update-many/example.md -->

Usa [`Model.updateOne(filter, changes)`](./update-one.md) cuando el filtro identifique un campo `@id` o `@unique`.
