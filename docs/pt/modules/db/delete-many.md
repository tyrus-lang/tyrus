---
title: deleteMany
description: Exclui todos os registros do modelo que correspondem a um filtro não vazio.
---

# Model.deleteMany

Chame `User.deleteMany(filter)` em um modelo importado. O filtro deve ser um objeto não vazio. Cada valor de filtro é parametrizado, e o compilador verifica os nomes dos campos e os tipos dos valores usando o modelo e o snapshot do schema.

A exclusão afeta todos os registros que correspondem ao filtro e retorna `Promise<Result<u64, DbError>>`; o valor de sucesso é a quantidade de registros excluídos. Falhas de constraint ou conexão com o banco são valores `DbError` recuperáveis.

Este fragmento pressupõe um modelo `User` importado com um campo `email`.

<!-- @include: ../../../.fragments/modules/db/delete-many/example.md -->

Use [`Model.deleteOne(filter)`](./delete-one.md) quando o filtro identificar um campo `@id` ou `@unique`.
