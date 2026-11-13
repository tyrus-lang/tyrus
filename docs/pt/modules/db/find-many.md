---
title: findMany
description: Consulte registros de modelos com filtros verificados e parametrizados.
---

# Model.findMany

Importe uma interface `@model` exportada e chame `Model.findMany(filter)` para buscar seus registros. O método retorna `Promise<Result<Model[], DbError>>` e aplica os mapeamentos físicos de tabela e coluna do modelo.

Aguarde o método antes de usar o `Result`. Os campos de um filtro plano são combinados com `AND`. Um valor escalar significa igualdade, como em `User.findMany({ active: true })`. Para comparações, use um objeto aninhado com um ou mais operadores entre `eq`, `ne`, `gt`, `gte`, `lt` e `lte`, por exemplo `User.findMany({ age: { gte: 18 } })`.

Os operadores de comparação significam: `eq` é igual a, `ne` é diferente de, `gt` é maior que, `gte` é maior ou igual a, `lt` é menor que e `lte` é menor ou igual a. Quando um campo tem mais de um operador, todas as condições precisam corresponder.

Use `or` com um array não vazio de objetos de filtro para corresponder a qualquer alternativa. Os campos de cada objeto são combinados com `AND`, os objetos são combinados com `OR`, e os campos ao lado de `or` continuam combinados com o grupo inteiro por `AND`.

Use `in` com um array não vazio para corresponder a qualquer valor listado. Em um padrão `like`, `%` corresponde a zero ou mais caracteres e `_` corresponde a um caractere: `"Ada"` corresponde exatamente a `Ada`, `"Ada%"` corresponde a valores que começam com `Ada`, `"%son"` corresponde a valores que terminam com `son` e `"%Ada%"` corresponde a valores que contêm `Ada` em qualquer posição. A correspondência segue o comportamento `LIKE` do backend configurado. Todos os valores continuam sendo parâmetros e são verificados em relação à coluna do modelo selecionado.

Este exemplo pressupõe que o modelo declare os campos `active`, `age` e `role`.

<!-- @include: ../../../.fragments/modules/db/find-many/or-filter.md -->

## Carregamento de relações

Declare relações de modelo com `@belongsTo("foreignKeyField")`, `@hasOne("foreignKeyField")`, `@hasMany("foreignKeyField")` ou `@manyToMany("pivotTable", "sourceKey", "targetKey")` e passe seus nomes em `{ include: ["team"] }`. Selecione um subconjunto de campos escalares com `{ include: [{ team: ["id", "name"] }] }`; o tipo do resultado reflete os campos selecionados. Includes sem projeção retornam o Model relacionado completo. O carregamento é explícito, usa uma instrução SQL e aceita somente um nível. Incluir uma relação `hasMany` ou `manyToMany` exige `relationLimit` de 1 a 500, que limita cada coleção incluída. Coleções relacionadas são ordenadas pelo `@id` do Model relacionado para manter resultados estáveis.

<!-- @include: ../../../.fragments/modules/db/find-many/relationships.md -->

## Paginação numerada

Passe `{ paginate: true, pageSize, page? }` como segundo argumento para retornar uma página numerada. A página padrão é 1. `Page<T>` contém `items`, `total`, `pageSize`, `page` e `totalPages`; `total` conta todos os registros correspondentes ao filtro antes da paginação. O MongoDB oferece paginação numerada ordenando por `_id` e aplicando o deslocamento solicitado. Importe `Page` de `tyrus:db` quando citar esse resultado na assinatura de uma função.

<!-- @include: ../../../.fragments/modules/db/find-many/pagination.md -->

## Paginação por cursor

Defina `cursor: true` para usar paginação por cursor. Omita `after` na primeira chamada; na próxima, passe o `nextCursor` da página anterior sem alterações. O cursor é o valor serializado do `@id` da última linha retornada, não a quantidade de itens nem o número da página. Para um `@id` inteiro, `after: "10"` continua após o id 10, não no décimo resultado. Os resultados são ordenados pelo `@id` do modelo, e `hasMore` indica se existe outra página. `total` continua contando todos os registros correspondentes, independentemente da posição do cursor. O MongoDB usa o `_id` gerado para ordenar e comparar o cursor, cujo token é o valor serializado de ObjectId de 24 caracteres retornado na página.

O compilador verifica cada campo em relação ao modelo e valida a consulta gerada e os tipos dos parâmetros contra o snapshot local do schema do backend relacional configurado. O MongoDB usa a mesma API de filtros do modelo, mas não usa snapshot de schema relacional. A contagem e os itens da página são lidos separadamente, então gravações concorrentes podem alterar o total observado entre chamadas de página. Os valores são vinculados por placeholders; os identificadores de tabela e coluna vêm apenas da declaração do modelo.

As mutações do modelo estão documentadas na página [Modelos](./models.md). Use [`findOne`](./find-one.md) para retornar um registro ou um valor de ausência tipado.
