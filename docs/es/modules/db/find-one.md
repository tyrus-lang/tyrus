---
title: findOne
description: Busca un registro del modelo con un filtro comprobado y parametrizado.
---

# Model.findOne

Llama a `User.findOne(filter)` en un modelo importado. El método usa los mismos campos y operadores de filtro que [`findMany`](./find-many.md), enlaza cada valor como parámetro y comprueba la consulta generada con el snapshot local del esquema SQLite.

El tipo de retorno es `Promise<Result<Optional<User, void>, DbError>>`. Espera la llamada para recibir el `Result`, que contiene los errores de la base de datos. Si hay éxito, el `Optional` interno contiene el registro coincidente o está ausente cuando no hay coincidencias. Compruébalo con `if (user)` o proporciona un valor alternativo con `??`.

La consulta usa `LIMIT 1`. Si el filtro puede coincidir con varias filas, la fila seleccionada no es determinista, salvo que el filtro identifique una fila única, por ejemplo, mediante `@id` o `@unique`.

Este fragmento supone que `User` es una interfaz `@model` importada con un campo `email`.

<!-- @include: ../../../.fragments/modules/db/find-one/example.md -->

Usa [`findMany`](./find-many.md) cuando el resultado deba ser una lista, incluso en el caso de una lista vacía. Los errores de la base de datos siguen siendo valores `DbError` recuperables dentro del `Result` recibido tras el await.
