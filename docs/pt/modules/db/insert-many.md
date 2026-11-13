---
title: insertMany
description: Insere um lote limitado de registros do modelo.
---

# Model.insertMany

Chame `User.insertMany(rows)` em um modelo importado para inserir vários registros em uma única operação do backend. Todas as linhas devem informar os mesmos campos na mesma ordem. Os backends SQL vinculam os valores como parâmetros, e uma falha de constraint rejeita a operação com um `DbError` recuperável.

O tipo de retorno é `Promise<Result<User[], DbError>>`; aguarde a chamada antes de usar o `Result`. Em caso de sucesso, os registros inseridos são retornados como `User[]`, incluindo identificadores e timestamps gerados. O lote deve conter de 1 a 500 registros. SQLite, PostgreSQL e MySQL usam o limite conservador de 999 parâmetros SQL vinculados do Tyrus, então o limite efetivo de linhas é `min(500, floor(999 / quantidadeDeCampos))`; por exemplo, linhas com seis campos informados permitem no máximo 166 registros por chamada. O MongoDB envia documentos BSON em vez de parâmetros SQL, portanto esse limite não se aplica; são aceitos até 500 documentos, respeitando o limite BSON de 16 MiB por documento.

Colunas obrigatórias, não nullable e sem default no banco precisam ser informadas em todas as linhas. Colunas nullable, colunas com default no snapshot local do banco e a única chave primária `@id` do modelo podem ser omitidas. Uma chave primária omitida só é gerada quando o schema da tabela configurada define uma identidade ou um default para ela.

Os backends SQL executam o lote como uma única instrução. O MongoDB valida todos os documentos antes de enviar o lote e usa uma operação de inserção ordenada, mas uma falha no servidor ainda pode deixar documentos inseridos antes da falha; `insertMany` não promete rollback entre backends. Em caso de sucesso, todos os recursos inseridos são retornados na ordem de entrada, incluindo os identificadores gerados. O MongoDB ainda pode retornar uma falha recuperável depois de progresso parcial no servidor.

Este fragmento pressupõe um modelo `User` importado com um campo `email`.

<!-- @include: ../../../.fragments/modules/db/insert-many/example.md -->

Use [`Model.insertOne(data)`](./insert-one.md) para inserir um único registro.
