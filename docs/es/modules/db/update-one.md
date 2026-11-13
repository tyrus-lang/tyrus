---
title: updateOne
description: Actualiza un registro del modelo seleccionado por un campo único.
---

# Model.updateOne

Llama a `User.updateOne(filter, changes)` en un modelo importado. El filtro debe contener una condición de igualdad para un campo marcado con `@id` o `@unique`, y `changes` debe contener al menos un campo conocido del modelo. El campo `@id` no se puede modificar.

La actualización está parametrizada y devuelve `Promise<Result<User?, DbError>>`; espera la llamada antes de usar el `Result`. Si tiene éxito, devuelve el `User` actualizado o un valor ausente cuando no hubo coincidencias. Los errores de constraints o conexión con la base de datos son valores `DbError` recuperables. El requisito de un filtro único evita que la operación coincida con varios registros.

Este fragmento supone un modelo `User` importado con `id` marcado con `@id` y un campo `email`.

<!-- @include: ../../../.fragments/modules/db/update-one/example.md -->

Usa [`Model.updateMany(filter, changes)`](./update-many.md) cuando el filtro pueda coincidir con varios registros.
