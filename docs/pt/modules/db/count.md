---
title: count
description: Conta registros do modelo que correspondem a um filtro.
---

# count

Chame `Model.count(filter)` para contar os registros do modelo que correspondem a um filtro. O resultado é `Promise<Result<u64, DbError>>`; aguarde a chamada antes de usar o `Result`.

Passe um objeto vazio, `Model.count({})`, para contar todos os registros da tabela do modelo.

## Exemplo

<!-- @include: ../../../.fragments/modules/db/count/example.md -->
