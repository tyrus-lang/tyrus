---
title: updateMany
description: Atualiza todos os registros do modelo que correspondem a um filtro não vazio.
---

# Model.updateMany

Chame `User.updateMany(filter, changes)` em um modelo importado. O filtro deve ser um objeto não vazio, e `changes` deve conter pelo menos um campo conhecido do modelo. O campo `@id` não pode ser alterado.

A atualização é parametrizada e afeta todos os registros que correspondem ao filtro. Ela retorna `Promise<Result<u64, DbError>>`; o valor de sucesso é a quantidade de registros atualizados. Falhas de constraint ou conexão com o banco são valores `DbError` recuperáveis.

Este fragmento pressupõe um modelo `User` importado com `id` marcado com `@id` e os campos `email` e `displayName`.

<!-- @include: ../../../.fragments/modules/db/update-many/example.md -->

Use [`Model.updateOne(filter, changes)`](./update-one.md) quando o filtro identificar um campo `@id` ou `@unique`.
