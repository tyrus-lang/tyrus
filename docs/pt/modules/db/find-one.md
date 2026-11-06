---
title: findOne
description: Busca um registro do modelo com um filtro verificado e parametrizado.
---

# Model.findOne

Chame `User.findOne(filter)` em um model importado. O método usa os mesmos campos e operadores de filtro de [`findMany`](./find-many.md), vincula cada valor como parâmetro e verifica a consulta gerada contra o snapshot local do schema SQLite.

O tipo de retorno é `Promise<Result<Optional<User, void>, DbError>>`. Aguarde a chamada para receber o `Result`, que carrega erros do banco. Em caso de sucesso, o `Optional` interno contém o registro correspondente ou fica ausente quando nenhum registro corresponde. Faça narrowing com `if (user)` ou forneça um valor alternativo com `??`.

A consulta usa `LIMIT 1`. Quando o filtro pode corresponder a várias linhas, a linha selecionada não é determinística, a menos que o filtro identifique uma linha única, por exemplo, usando `@id` ou `@unique`.

Este fragmento pressupõe que `User` é uma interface `@model` importada com um campo `email`.

<!-- @include: ../../../.fragments/modules/db/find-one/example.md -->

Use [`findMany`](./find-many.md) quando o resultado deve ser uma lista, inclusive no caso de lista vazia. Falhas do banco continuam sendo valores `DbError` recuperáveis dentro do `Result` recebido após o await.
