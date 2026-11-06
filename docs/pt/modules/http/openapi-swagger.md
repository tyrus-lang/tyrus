---
title: OpenAPI / Swagger
description: Gere e configure a documentação da API com OpenAPI e Swagger UI.
---

# OpenAPI / Swagger

`app.docs()` pertence a `tyrus:http`. Ele monta um documento OpenAPI 3.2 a partir das rotas registradas no router e serve uma página Swagger UI pela própria aplicação. Não é necessário um serviço externo de documentação.

## Ative a documentação

Chame `app.docs(options)` no router. A página padrão é `/docs`, e o documento OpenAPI fica disponível em `/docs/openapi.json`. Defina `path` para usar outro caminho base. As rotas registradas antes ou depois dessa chamada são incluídas.

`app.docs(options)` retorna `Result<void, HttpError>` porque o runtime pode recusar uma configuração de documentação inválida. Propague o resultado com `?` durante a configuração do router ou trate-o com `match`.

| Opção | Tipo | Padrão | Finalidade |
| --- | --- | --- | --- |
| `path` | `string` | `"/docs"` | Caminho base da Swagger UI, do documento OpenAPI e dos recursos incluídos. Deve começar com `/`. |
| `title` | `string` | `"API"` | Título da API exibido pela Swagger UI e no objeto OpenAPI `info`. |
| `version` | `string` | `"1.0.0"` | Versão da API gravada no objeto OpenAPI `info`. |
| `description` | `string` | vazio | Descrição da API gravada no objeto OpenAPI `info`. |
| `auth` | objeto literal | nenhum | Esquemas de segurança exibidos pela Swagger UI e declarados no documento OpenAPI. Não autentica requisições por conta própria. |
| `css` | `string` | vazio | Código CSS inserido na página após a folha de estilos incluída da Swagger UI. Passe o texto CSS, não o caminho de um arquivo. |
| `preHandler` | array de funções nomeadas | nenhum | Guards síncronos executados antes de servir a página, o documento OpenAPI e os recursos da Swagger UI. |

O compilador valida o objeto inline. Não existe uma interface nomeada para importar. Essas são todas as opções aceitas atualmente por `app.docs`.

## Conteúdo do documento

O documento descreve o método e o caminho de cada rota, parâmetros de caminho, os schemas declarados de `params`, `query`, `headers` e `cookies`, corpos de requisição, status de resposta e schemas de resposta. Interfaces e enums viram schemas nomeados reutilizáveis. Corpos JSON de requisição incluem exemplos gerados que respeitam as restrições declaradas no schema. `Instant` é descrito como uma string de data e hora.

Adicione `tags` às opções da rota para agrupar operações em seções nomeadas na Swagger UI. Rotas sem tags aparecem em `default`.

```tyrus
router.get("/users", {
    tags: ["users"],
    response: { 200: User[] },
    handler: listUsers,
});
```

O método HTTP `QUERY` é representado como uma operação OpenAPI 3.2, incluindo o corpo de requisição declarado. Rotas WebSocket aparecem como operações `GET` com resposta `101 Switching Protocols` e a extensão `x-tyrus-websocket`. O documento é montado a partir das declarações das rotas, sem exigir uma definição OpenAPI separada para cada uma.

## Configure a autenticação da Swagger UI

Use `auth` para descrever como um cliente se autentica na Swagger UI. Os esquemas aparecem no controle **Authorize** e são gravados nas definições de segurança do OpenAPI. A descrição do esquema não é uma verificação de acesso: proteja as rotas da API com a lógica de autenticação e autorização da aplicação.

### Token bearer

```tyrus
auth: { bearer: true, bearerFormat: "JWT" }
```

`bearerFormat` é opcional e só pode ser usado com `bearer: true`.

### Chave de API

```tyrus
auth: { apiKey: { name: "x-api-key", in: "header" } }
```

`name` é o nome do header, parâmetro de query ou cookie da chave. `in` deve ser `"header"`, `"query"` ou `"cookie"`; prefira um header para credenciais.

## Personalize a página com CSS

`css` recebe código CSS, não o caminho de uma folha de estilos. O runtime o coloca em um elemento `<style>` depois da folha de estilos da Swagger UI, então essas declarações podem substituir os estilos padrão.

```tyrus
css: ".swagger-ui .topbar { background: #16324f; } .swagger-ui .info { margin: 2rem 0; }"
```

## Proteja os endpoints da documentação

`preHandler` recebe o `HttpRequest` bruto e deve ser um array literal de funções síncronas nomeadas com a assinatura `(request: HttpRequest) => HttpResponse | true`. Retorne `true` para servir o recurso de documentação solicitado. Retorne um `HttpResponse` para interromper a requisição, por exemplo com `HttpStatus.Forbidden`. O mesmo guard protege a página HTML, `/openapi.json`, a folha de estilos e o bundle de scripts.

Use dentro do guard o validador de sessão ou credenciais real da aplicação. Este exemplo pressupõe que `isAuthorizedAdmin` é uma função já existente na aplicação; ela não é fornecida por `tyrus:http`.

<!-- @include: ../../../.fragments/modules/http/openapi-swagger/pre-handler-example.md -->

A opção `auth` acima apenas informa à Swagger UI como enviar credenciais ao testar uma operação. Ela não protege a página da documentação, não valida credenciais e não substitui a autorização das rotas. Use `preHandler` para controlar o acesso à própria documentação e as verificações normais de autorização da aplicação para proteger as operações da API.

## Exemplo completo de configuração

Este exemplo combina os metadados da API, um esquema bearer, CSS personalizado e o guard da documentação. Substitua o título e o esquema de autorização de exemplo pelos usados pela aplicação.

<!-- @include: ../../../.fragments/modules/http/openapi-swagger/setup-example.md -->
