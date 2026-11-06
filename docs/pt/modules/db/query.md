---
title: query
description: Executa consultas relacionais tipadas e mutações parametrizadas.
---

# db.query

<SignatureBlock label="Assinatura">query&lt;T&gt;(sql: string, parameters: unknown[]): Promise&lt;Result&lt;T[], DbError&gt;&gt;</SignatureBlock>

<SignatureBlock label="Assinatura">query(sql: string, parameters: unknown[]): Promise&lt;Result&lt;void, DbError&gt;&gt;</SignatureBlock>

`db.query<T>` retorna `Promise<Result<T[], DbError>>` para instruções `SELECT` validadas e decodifica cada linha retornada na interface `T`, que deve ser citada explicitamente. Aguarde toda chamada ao banco; uma consulta sem linhas correspondentes resolve para um array vazio dentro de `Ok`. Para `INSERT`, `UPDATE` e `DELETE`, chame `db.query` sem argumento de tipo; uma mutação bem-sucedida resolve para `void` dentro de `Ok`.

Um `Instant` enviado como parâmetro SQL é vinculado usando a representação do backend selecionado. O SQLite armazena texto UTC, o PostgreSQL usa `TIMESTAMPTZ` e rejeita valores com precisão inferior a microssegundos, e o MySQL usa `DATETIME(3)` e rejeita valores com precisão inferior a milissegundos. Um `SELECT *` tipado decodifica campos `Instant` do Model para a representação Tyrus.

`COUNT`, `SUM`, `AVG`, `MIN` e `MAX` são aceitos em `SELECT`. Consulte [Agregações SQL](./aggregates.md) para tipos de resultado, aliases, exemplos e o comportamento sem correspondências.

Dentro de uma string de `db.query`, o Language Server completa expressões de agregação, cláusulas SQL, tabelas e colunas do snapshot local do schema. Passe o cursor sobre uma tabela ou coluna para ver o schema, ou sobre o nome de uma agregação para ver sua assinatura e o comportamento do resultado.

Para inspecionar uma consulta no editor, posicione o cursor dentro da string SQL, abra as Ações de Código e escolha **Explain SQL query**. O resultado inclui o SQL normalizado, os tipos dos parâmetros com os valores ocultos e os campos retornados; a validação usa o snapshot local sem abrir conexão com o banco. A explicação no editor aceita strings SQL de até 64 KiB.

Joins internos e externos exigem que os campos de resultado reflitam o lado que pode estar ausente. Consulte [Joins SQL](./joins.md).

<!-- @include: ../../../.fragments/modules/db/query/example.md -->

O compilador valida o tipo da instrução, as colunas selecionadas, os aliases, a quantidade e os tipos dos parâmetros com o snapshot local do schema. Ele rejeita arrays com mais de 999 parâmetros usando `E3305`; o runtime aplica o mesmo limite e retorna `ERR_DB_PARAMETER_LIMIT` como proteção final. O SQL deve ser um literal de string e os valores devem ser enviados em um literal de array separado. Consulte [Segurança SQL](./security.md) para o subconjunto de instruções aceito e as regras de vinculação.
