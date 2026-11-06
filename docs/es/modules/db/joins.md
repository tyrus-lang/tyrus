---
title: Joins SQL
description: Joins internos y externos con campos opcionales verificados en el resultado.
---

# Joins SQL

`db.query<T>` admite `JOIN`, `INNER JOIN`, `LEFT JOIN`, `RIGHT JOIN` y `FULL JOIN`. También admite `LEFT OUTER JOIN`, `RIGHT OUTER JOIN` y `FULL OUTER JOIN`. Cada cláusula `ON` compara una columna de la tabla recién unida con otra de una tabla anterior mediante `=`. Califica la columna cuando su nombre aparezca en más de una tabla.

| Join | Columnas que pueden faltar en una fila de resultado |
| --- | --- |
| `JOIN` o `INNER JOIN` | La tabla recién unida permanece presente; las tablas anteriores conservan su nulabilidad previa |
| `LEFT JOIN` | Tabla recién unida |
| `RIGHT JOIN` | Tablas anteriores |
| `FULL JOIN` | Tablas anteriores y tabla recién unida |

Declara como `T?` un campo de un lado que puede faltar, aunque la columna no sea nullable en el schema de la tabla. Con varios joins, un `RIGHT JOIN` o `FULL JOIN` posterior puede hacer opcionales las tablas anteriores. Una columna nullable en el schema también requiere `T?`. El checker no infiere un resultado no nulo a partir de un predicado `WHERE`; conserva el tipo opcional al filtrar filas del join.

## Ejemplo

Este ejemplo supone que el snapshot local del schema contiene `db_examples(id: i64, label: string)` y `db_example_roles(id: i64, title: string)`.

<!-- @include: ../../../.fragments/modules/db/joins/example.md -->

`SELECT *` solo se admite cuando cada campo solicitado del resultado corresponde a una única columna; usa proyecciones explícitas y alias cuando las tablas compartan nombres. `CROSS JOIN` y las expresiones arbitrarias en `ON` todavía no se admiten. Consulta [Seguridad SQL](./security.md) para las reglas de vinculación.
