---
title: API Flow
description: Acompanhe uma requisição de User pelo main, Model, schemas, routes, controllers, services e repository de uma API SQLite.
---

# API Flow

Este guia acompanha requisições de User por uma API SQLite, desde o `tyrus.json` e a inicialização do servidor até a operação SQL ou do Model e a resposta HTTP. O código usa a mesma estrutura e formatação do projeto `api-sqlite`.

## Estrutura do projeto

O diretório `src/routes/` é descoberto automaticamente quando `createRouter()` é executado. O Tyrus chama cada função de rota exportada com o router da aplicação, então `main.tyr` não precisa de uma importação para cada módulo de rota.

## Exemplo completo

O exemplo expõe endpoints de listagem, paginação numérica, busca, leitura, criação, atualização, substituição e exclusão. O Model fornece o `id` e os timestamps gerados; o repository usa `db.query` para SQL e métodos do Model para operações tipadas com registros.

<!-- @include: ../../../.fragments/modules/http/api-flow/complete-example.md -->

## Fluxo da requisição

- `main.tyr` cria o router, configura os metadados do Swagger e inicia o servidor com `app.listen`.
- `models/users.tyr` mapeia a interface `User` para a tabela `users`; o `id`, `createdAt` e `updatedAt` gerados não precisam ser declarados na interface.
- `schemas/user.tyr` define as entradas das rotas, os corpos de resposta e os corpos de erro; os erros da aplicação declaram `code` e `message`.
- `routes/user.tyr` declara os métodos HTTP, os schemas de validação e resposta e o controller de cada endpoint.
- `controllers/user.tyr` trata os status HTTP e converte o sucesso ou a falha de `Result` em corpos de resposta.
- `services/user.tyr` nomeia as operações da aplicação e retorna resultados assíncronos tipados.
- `repositories/user.tyr` controla o acesso ao banco de dados e retorna valores `Result`; usa parâmetros vinculados para SQL e operações de Model para os registros.

Os nomes e caminhos de arquivo do exemplo são ilustrativos; mantenha os módulos de rota em `src/routes/` para o registro automático. Consulte [registro automático de rotas](./automatic-routes.md), [`tyrus:db`](../db/) e [OpenAPI / Swagger](./openapi-swagger.md) para os contratos relacionados.
