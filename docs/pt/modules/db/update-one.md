---
title: updateOne
description: Atualiza um registro do modelo selecionado por um campo único.
---

# Model.updateOne

Chame `User.updateOne(filter, changes)` em um modelo importado. O filtro deve conter uma condição de igualdade para um campo marcado com `@id` ou `@unique`, e `changes` deve conter pelo menos um campo conhecido do modelo. O campo `@id` não pode ser alterado.

A atualização é parametrizada e retorna `Promise<Result<User?, DbError>>`; aguarde a chamada antes de usar o `Result`. Em caso de sucesso, retorna o `User` atualizado ou um valor ausente quando nenhum registro correspondeu. Falhas de constraint ou conexão com o banco são valores `DbError` recuperáveis. A exigência de um filtro único impede que a operação corresponda a vários registros.

Este fragmento pressupõe um modelo `User` importado com `id` marcado com `@id` e um campo `email`.

<!-- @include: ../../../.fragments/modules/db/update-one/example.md -->

Use [`Model.updateMany(filter, changes)`](./update-many.md) quando o filtro puder corresponder a vários registros.
