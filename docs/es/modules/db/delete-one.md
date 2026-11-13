---
title: deleteOne
description: Elimina un registro del modelo seleccionado por un campo único.
---

# Model.deleteOne

Llama a `User.deleteOne(filter)` en un modelo importado. El filtro debe contener una condición de igualdad para un campo marcado con `@id` o `@unique`; este requisito evita que la operación coincida con varios registros.

La eliminación está parametrizada y devuelve `Promise<Result<User?, DbError>>`; espera la llamada antes de usar el `Result`. Si tiene éxito, devuelve el `User` eliminado o un valor ausente cuando no hubo coincidencias. Los errores de constraints o conexión con la base de datos son valores `DbError` recuperables.

Este fragmento supone un modelo `User` importado cuyo campo `id` está marcado con `@id`.

<!-- @include: ../../../.fragments/modules/db/delete-one/example.md -->

Usa [`Model.deleteMany(filter)`](./delete-many.md) cuando el filtro pueda coincidir con varios registros.
