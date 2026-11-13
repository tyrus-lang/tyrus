---
title: insertOne
description: Insere um registro do modelo com valores vinculados.
---

# Model.insertOne

Chame `User.insertOne(data)` em um modelo importado para inserir um registro. As chaves do objeto são nomes de campos do modelo; o compilador as mapeia para as colunas declaradas no banco e vincula cada valor como parâmetro.

O tipo de retorno é `Promise<Result<User, DbError>>`; aguarde a chamada antes de usar o `Result`. Em caso de sucesso, o `User` inserido é retornado com o ID e os timestamps gerados. Falhas de constraint ou conexão com o banco são retornadas como `Err(error)`.

Colunas obrigatórias, não nullable e sem default no banco precisam ser informadas. Colunas nullable, colunas com default no snapshot local do banco e a única chave primária `@id` do modelo podem ser omitidas. Uma chave primária omitida só é gerada quando o schema da tabela configurada define uma identidade ou um default para ela.

Este fragmento pressupõe um modelo `User` importado com um campo `email`.

<!-- @include: ../../../.fragments/modules/db/insert-one/example.md -->

Use [`Model.insertMany(rows)`](./insert-many.md) para inserir um lote em uma única instrução.
