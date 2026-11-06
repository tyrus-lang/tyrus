---
title: Joins SQL
description: Joins internos e externos com campos opcionais conferidos no resultado.
---

# Joins SQL

`db.query<T>` aceita `JOIN`, `INNER JOIN`, `LEFT JOIN`, `RIGHT JOIN` e `FULL JOIN`. `LEFT OUTER JOIN`, `RIGHT OUTER JOIN` e `FULL OUTER JOIN` também são aceitos. Cada cláusula `ON` compara uma coluna da tabela recém-adicionada com uma coluna de uma tabela anterior usando `=`. Qualifique a coluna quando seu nome aparecer em mais de uma tabela.

| Join | Colunas que podem estar ausentes de uma linha de resultado |
| --- | --- |
| `JOIN` ou `INNER JOIN` | A tabela recém-adicionada permanece presente; tabelas anteriores mantêm a nulabilidade anterior |
| `LEFT JOIN` | Tabela recém-adicionada |
| `RIGHT JOIN` | Tabelas anteriores |
| `FULL JOIN` | Tabelas anteriores e tabela recém-adicionada |

Declare como `T?` um campo de um lado que pode estar ausente, mesmo que a coluna não seja nullable no schema da tabela. Com vários joins, um `RIGHT JOIN` ou `FULL JOIN` posterior pode tornar opcionais as tabelas anteriores. Uma coluna nullable no schema também exige `T?`. O checker não infere resultado não nulo a partir de um predicado `WHERE`; mantenha o tipo opcional ao filtrar linhas do join.

## Exemplo

Este exemplo pressupõe que o snapshot local do schema contém `db_examples(id: i64, label: string)` e `db_example_roles(id: i64, title: string)`.

<!-- @include: ../../../.fragments/modules/db/joins/example.md -->

`SELECT *` só é aceito quando cada campo solicitado do resultado corresponde a uma única coluna; use projeções explícitas e aliases quando as tabelas compartilharem nomes. `CROSS JOIN` e expressões arbitrárias em `ON` ainda não são aceitos. Consulte [Segurança SQL](./security.md) para as regras de vinculação.
