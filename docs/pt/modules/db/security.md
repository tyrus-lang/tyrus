---
title: Segurança SQL
description: Vinculação de parâmetros e superfície SQL aceita por tyrus:db.
---

# Segurança SQL

`db.query<T>` para `SELECT` e `db.query` para mutações recebem literais de string SQL e um literal de array separado para os valores. Cada placeholder `?` é vinculado como valor pelo driver relacional configurado; os valores dos parâmetros nunca são concatenados ao texto SQL.

Placeholders não representam nomes de tabela, nomes de coluna, operadores nem fragmentos SQL. Expressões de SQL dinâmico e concatenação são recusadas por essas APIs. Quando a aplicação precisar de poucas formas de consulta, escolha entre alternativas completas de SQL literal no fluxo do programa.

O compilador valida os comandos aceitos com o snapshot local do schema. `db.query` aceita um único `INSERT`, `UPDATE` ou `DELETE` de uma tabela; `UPDATE` e `DELETE` exigem `WHERE`. `db.query<T>` aceita o subconjunto documentado de `SELECT`, incluindo [joins internos e externos](./joins.md) com comparações de igualdade em `ON`. `CROSS JOIN` ainda não é aceito. Qualifique as colunas com o nome ou alias da tabela quando mais de uma tabela do join tiver esse nome; referências ambíguas são recusadas.

[`GROUP BY`](./aggregates.md#group-by) aceita colunas de agrupamento explícitas e valida cada projeção comum em relação a elas. `HAVING` é aceito para filtrar grupos após a agregação e pode referenciar colunas agrupadas ou aliases de agregação. Veja o [exemplo de GROUP BY](./aggregates.md#group-by), que usa um parâmetro vinculado em `HAVING`.

Em `SELECT *`, a geração de código substitui `*` pelos campos escapados da interface de resultado solicitada, para que o backend relacional configurado retorne apenas as colunas necessárias ao decoder. Com joins, cada campo solicitado deve corresponder a uma única coluna sem ambiguidade; caso contrário, selecione as colunas explicitamente e use aliases. Projeções explícitas podem usar aliases, mas os nomes de saída devem corresponder de forma única e exata aos campos da interface de resultado. Colunas desconhecidas, campos ausentes ou extras, aliases ambíguos, expressões não suportadas e quantidades ou tipos incompatíveis de parâmetros são recusados durante a validação.

Essas restrições e os parâmetros vinculados impedem que valores de consulta alterem a estrutura SQL pelas APIs de `tyrus:db`. Continue validando autorização e regras de negócio no código da aplicação; a validação do schema não decide se quem chamou pode acessar uma linha.

Use [`tyrus db explain-sql`](/pt/tooling/cli#comandos-de-schema-do-banco) com SQL enviado pelo stdin para inspecionar o SQL normalizado e os tipos de parâmetros inferidos. O comando valida pelo snapshot local sem abrir o banco, oculta literais string e numéricos e nunca exibe valores vinculados.
