---
title: Agregações SQL
description: COUNT, SUM, AVG, MIN e MAX em consultas tipadas ao banco de dados.
---

# Agregações SQL

Use expressões de agregação na lista de `SELECT` de [`db.query<T>`](./query.md). Declare cada campo de resultado em `T` com o tipo indicado abaixo e dê a cada expressão um alias explícito com `AS` que corresponda ao campo.

| Expressão | Argumento | Campo de resultado |
| --- | --- | --- |
| `COUNT(*) AS total` | Todas as linhas correspondentes | `total: i64` |
| `COUNT(column) AS total` | Valores presentes na coluna | `total: i64` |
| `SUM(column) AS sum` | Coluna `i64` ou `f64` | `sum: i64?` ou `sum: f64?` |
| `AVG(column) AS average` | Coluna `i64` ou `f64` | `average: f64?` |
| `MIN(column) AS minimum` | Coluna `i64`, `f64` ou `string` | Tipo opcional da coluna |
| `MAX(column) AS maximum` | Coluna `i64`, `f64` ou `string` | Tipo opcional da coluna |

`COUNT(*)` conta linhas; `COUNT(column)` conta linhas com valor nessa coluna. Ambos retornam `0` quando não há valores correspondentes. As demais agregações retornam um opcional vazio nesse caso; use `??` ou faça narrowing antes de usar o valor. Uma consulta de agregação sem `GROUP BY` retorna uma linha mesmo quando nenhuma linha de origem corresponde.

## Exemplo

Este exemplo pressupõe que o snapshot local do schema contém `db_examples(id: i64)` e que o banco de dados em execução tem a mesma tabela.

<!-- @include: ../../../.fragments/modules/db/aggregates/example.md -->

É possível combinar várias agregações em um `SELECT`.

## GROUP BY

Use `GROUP BY` para combinar colunas comuns e agregações. Toda coluna comum selecionada deve aparecer na lista de agrupamento, pelo nome da coluna, pelo nome qualificado ou pelo alias explícito de `SELECT`. `SELECT *` não é aceito com `GROUP BY`; liste as colunas de resultado explicitamente. `HAVING` filtra grupos após a agregação e pode se referir a colunas agrupadas ou aliases de agregação. Vincule valores por parâmetros `?`. `ORDER BY` pode se referir a uma coluna agrupada ou a um alias de agregação. O resultado contém uma linha por grupo ou um array vazio se não houver grupos correspondentes.

Este exemplo retorna uma linha para cada valor de `id`:

<!-- @include: ../../../.fragments/modules/db/aggregates/group-by.md -->

O SQL deve ser um literal de string, e valores dinâmicos pertencem ao array separado de parâmetros.
