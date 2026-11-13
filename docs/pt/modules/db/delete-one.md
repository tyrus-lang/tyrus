---
title: deleteOne
description: Exclui um registro do modelo selecionado por um campo único.
---

# Model.deleteOne

Chame `User.deleteOne(filter)` em um modelo importado. O filtro deve conter uma condição de igualdade para um campo marcado com `@id` ou `@unique`; essa exigência impede que a operação corresponda a vários registros.

A exclusão é parametrizada e retorna `Promise<Result<User?, DbError>>`; aguarde a chamada antes de usar o `Result`. Em caso de sucesso, retorna o `User` excluído ou um valor ausente quando nenhum registro correspondeu. Falhas de constraint ou conexão com o banco são valores `DbError` recuperáveis.

Este fragmento pressupõe um modelo `User` importado cujo campo `id` está marcado com `@id`.

<!-- @include: ../../../.fragments/modules/db/delete-one/example.md -->

Use [`Model.deleteMany(filter)`](./delete-many.md) quando o filtro puder corresponder a vários registros.
