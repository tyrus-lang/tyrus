---
title: Agregaciones SQL
description: COUNT, SUM, AVG, MIN y MAX en consultas tipadas a la base de datos.
---

# Agregaciones SQL

Usa expresiones agregadas en la lista `SELECT` de [`db.query<T>`](./query.md). Declara cada campo de resultado en `T` con el tipo indicado abajo y asigna a cada expresión un alias explícito con `AS` que coincida con el campo.

| Expresión | Argumento | Campo de resultado |
| --- | --- | --- |
| `COUNT(*) AS total` | Todas las filas coincidentes | `total: i64` |
| `COUNT(column) AS total` | Valores presentes en la columna | `total: i64` |
| `SUM(column) AS sum` | Columna `i64` o `f64` | `sum: i64?` o `sum: f64?` |
| `AVG(column) AS average` | Columna `i64` o `f64` | `average: f64?` |
| `MIN(column) AS minimum` | Columna `i64`, `f64` o `string` | Tipo opcional de la columna |
| `MAX(column) AS maximum` | Columna `i64`, `f64` o `string` | Tipo opcional de la columna |

`COUNT(*)` cuenta filas; `COUNT(column)` cuenta filas con un valor en esa columna. Ambos devuelven `0` cuando no hay valores coincidentes. Las demás agregaciones devuelven un opcional vacío en ese caso; usa `??` o estrecha el tipo antes de usar el valor. Una consulta agregada sin `GROUP BY` devuelve una fila aunque no coincida ninguna fila de origen.

## Ejemplo

Este ejemplo supone que el snapshot local del schema contiene `db_examples(id: i64)` y que la base de datos en ejecución tiene la misma tabla.

<!-- @include: ../../../.fragments/modules/db/aggregates/example.md -->

Puedes combinar varias agregaciones en un `SELECT`.

## GROUP BY

Usa `GROUP BY` para combinar columnas normales y agregaciones. Toda columna normal seleccionada debe aparecer en la lista de agrupación, mediante su nombre, su nombre calificado o su alias explícito de `SELECT`. `SELECT *` no se admite con `GROUP BY`; enumera las columnas de resultado de forma explícita. `HAVING` filtra grupos después de la agregación y puede referirse a columnas agrupadas o alias de agregación. Vincula los valores mediante parámetros `?`. `ORDER BY` puede referirse a una columna agrupada o a un alias de agregación. El resultado contiene una fila por grupo o un array vacío si no hay grupos coincidentes.

Este ejemplo devuelve una fila por cada valor de `id`:

<!-- @include: ../../../.fragments/modules/db/aggregates/group-by.md -->

El SQL debe ser un literal de cadena, y los valores dinámicos van en el array separado de parámetros.
