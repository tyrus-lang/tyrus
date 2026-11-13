---
title: count
description: Cuenta registros del modelo que coinciden con un filtro.
---

# count

Llama a `Model.count(filter)` para contar los registros del modelo que coinciden con un filtro. El resultado es `Promise<Result<u64, DbError>>`; espera la llamada antes de usar el `Result`.

Pasa un objeto vacío, `Model.count({})`, para contar todos los registros de la tabla del modelo.

## Ejemplo

<!-- @include: ../../../.fragments/modules/db/count/example.md -->
