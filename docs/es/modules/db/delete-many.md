---
title: deleteMany
description: Elimina todos los registros del modelo que coinciden con un filtro no vacío.
---

# Model.deleteMany

Llama a `User.deleteMany(filter)` en un modelo importado. El filtro debe ser un objeto no vacío. Cada valor del filtro se parametriza, y el compilador comprueba los nombres de campo y los tipos de valor con el modelo y el snapshot del schema.

La eliminación se aplica a todos los registros que coinciden con el filtro y devuelve `Promise<Result<u64, DbError>>`; el valor de éxito es la cantidad de registros eliminados. Los errores de constraints o conexión con la base de datos son valores `DbError` recuperables.

Este fragmento supone un modelo `User` importado con un campo `email`.

<!-- @include: ../../../.fragments/modules/db/delete-many/example.md -->

Usa [`Model.deleteOne(filter)`](./delete-one.md) cuando el filtro identifique un campo `@id` o `@unique`.
