---
title: Transações
description: Handles explícitos de transação SQL para agrupar consultas e operações de Model.
---

# Transações de banco de dados

Use `db.beginTransaction()` para iniciar uma transação, passe o handle como `{ transaction }` para `db.query` ou uma operação de Model sem paginação e finalize com `db.commitTransaction(transaction)` ou `db.rollbackTransaction(transaction)`. Cada chamada é assíncrona e retorna `Promise<Result<T, DbError>>`; trate cada resultado com `onfail`, `?` ou pattern matching.

`db.beginTransaction()` retorna `Promise<Result<DbTransaction, DbError>>`; commit e rollback retornam `Promise<Result<boolean, DbError>>`, em que `true` confirma que a transação foi finalizada. Importe `DbTransaction` de `tyrus:db` somente ao escrevê-lo em uma anotação de tipo explícita; handles inferidos não precisam de um import separado.

## Escopo

O handle da transação mantém consultas SQL e operações de Model sem paginação na mesma conexão e transação para SQLite, PostgreSQL ou MySQL. Passe `{ transaction }` como objeto de opções para `findOne` e `count`, inclua-o no objeto de opções existente de `findMany` e passe-o como último argumento de opções para mutações do Model. `findMany` paginado gerencia sua própria transação de leitura consistente e rejeita um handle explícito. Transações MongoDB não estão disponíveis nesta API.

As transações são explícitas: uma sequência bem-sucedida deve executar commit, enquanto uma falha detectada pela aplicação deve executar rollback antes de retornar o erro. Se uma consulta SQL ou operação de Model com o handle retornar um erro, o Tyrus executa rollback e encerra o handle automaticamente; operações posteriores com esse handle retornam `ERR_DB_INVALID_TRANSACTION`. Se uma transação for abandonada sem nenhuma dessas operações, o runtime executa rollback quando o serviço de banco de dados é encerrado; não use isso como fluxo normal.

O handle é opaco e deve ser passado de volta sem alterações. Ele não é um ID de linha nem um token de segurança, e aplicações não devem expô-lo a clientes.

## Exemplo com Model

Passe `{ transaction }` como último argumento de opções para uma mutação. Leituras podem usar o mesmo handle no objeto de opções existente.

<!-- @include: ../../../.fragments/modules/db/transactions/example.md -->
