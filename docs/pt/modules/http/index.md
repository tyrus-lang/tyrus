---
title: tyrus:http
description: APIs HTTP nativas para services tyrus e projetos API gerados.
---

# tyrus:http

`tyrus:http` é a superfície HTTP nativa para servidor e rotas. Ele foi pensado para projetos API gerados ao escolher o target API nas perguntas de `tyrus new` e para services explícitos que precisam de helpers tipados de request e response.

## Imports

<!-- @include: ../../../.fragments/modules/http/index/imports.md -->

## Itens exportados

| Item | Assinatura | Finalidade |
|------|------------|------------|
| [`createRouter`](./create-router.md) | `() => HttpRouter` | Cria um router HTTP nativo. |
| [API Flow](./api-flow.md) | Exemplo completo de API User | Acompanha o main, o Model, os schemas, as rotas, os controllers, os services e o repository em um único fluxo com SQLite. |
| [`OpenAPI / Swagger`](./openapi-swagger.md) | `app.docs(options)` | Gera o documento OpenAPI e serve a Swagger UI. |
| [Registro automático de rotas](./automatic-routes.md) | Descobre módulos de rota em `src/routes/` sem um registro manual. |
| [`listen`](./listen.md) | `app.listen(port, options?)` | Inicia o servidor e retorna `Result<HttpServer, HttpError>`. O runtime o mantém ativo depois que `main` retorna. |
| [`HttpStatus`](./http-status.md) | `enum HttpStatus` | Códigos HTTP nomeados. |
| [`HttpReply`](./http-reply.md) | `type HttpReply<T>` | API de reply tipada para controllers. |
| [`StreamBody`](./stream-body.md) | `type StreamBody` | Stream de bytes limitado e opt-in para corpo ou resposta de rota. |
| `HttpRouter` | `type HttpRouter` | Router nativo retornado por `createRouter`. |
| [`onResponse`](./on-response.md) | `(hook: (access: HttpAccessLog) => void) => void` | Registra um observador opcional de respostas do servidor. |
| [`HttpAccessLog`](./http-access-log.md) | `interface HttpAccessLog` | Metadados imutáveis de uma resposta concluída. |
| `HttpRequest` | `interface HttpRequest` | Requisição bruta disponível para pré-handlers e rotas brutas. |
| [`HttpResponse`](./http-response.md) | `interface HttpResponse` | Resposta bruta usada por pré-handlers e rotas brutas. |
| `HttpCacheOptions` | `interface HttpCacheOptions` | Configuração de ETag passada por `HttpReplyOptions`. |
| `HttpReplyOptions` | `interface HttpReplyOptions` | Metadados opcionais passados como o segundo argumento de `reply.send`. |
| [`CorsOptions`](./cors-options.md) | `interface CorsOptions` | Configuração completa passada para `app.cors(...)`. |
| [`CsrfOptions`](./csrf-options.md) | `interface CsrfOptions` | Nomes do cookie e header double-submit passados para `app.csrf(...)`. |
| [`CookieOptions`](./cookie-options.md) | `interface CookieOptions` | Atributos opcionais passados para `reply.setCookie(...)`. |
| [`HttpCookie`](./cookie-options.md) | `interface HttpCookie` | Um cookie de `reply.setCookies([...])`: `name`, `value` e `options` opcional. |
| [`ServerOptions`](./server-options.md) | `interface ServerOptions` | Host, limites, ciclo de vida, logging opcional e TLS passados para `app.listen(...)`. |
| `HttpError` | `interface HttpError` | Falha recuperável de inicialização retornada por `app.listen(...)`, com um `code` estável e uma `message` legível. |
| `HttpServer` | `type HttpServer` | Servidor em execução devolvido por `app.listen(...)`, com `port()` e `close()`. |
| [`WebSocket`](./websocket.md) | `type WebSocket` | Uma conexão WebSocket aberta, com `stream()`, `receive()`, `sendText(text)`, `sendBytes(data)`, `close()`, `protocol()` e `closeCode()`. |
| `WebSocketMessage` | `interface WebSocketMessage` | Uma mensagem WebSocket inteira: `binary`, e seu `text` ou seus `bytes`. |
| [`connectWebSocket`](./websocket.md) | `(url: string, protocols?: string[]) => Promise<Result<WebSocket, HttpError>>` | Abre uma conexão WebSocket para uma URL `ws://` ou `wss://`. |
| `fetch` | `(url: string, options?: FetchOptions) => Promise<Result<HttpClientResponse, HttpError>>` | Envia uma requisição e lê a resposta. |
| `HttpClientResponse` | `type HttpClientResponse` | A resposta que um `fetch` recebeu, com `status()`, `ok()`, `header(name)`, `text()`, `bytes()`, `json<T>()` e `close()`. |
| [`HttpClientStreamResponse`](./http-client-stream-response.md) | `type HttpClientStreamResponse` | O cabeçalho de uma resposta `fetch.stream(...)`, com `stream()` para seu corpo progressivo. |
| `FetchOptions` | `interface FetchOptions` | Headers, corpo de texto ou stream, timeout, limite de corpo, contagem de redirecionamentos e descompressão da resposta de um `fetch`. |
| `FetchPolicyOptions` | `interface FetchPolicyOptions` | Para onde o `fetch` pode ir, passado a `app.fetchPolicy(...)`. |
| `HttpHeader` | `interface HttpHeader` | Um header de requisição, escrito como `{ name, value }`. |

O `fetch` é o cliente. Ele envia uma requisição e lê a resposta, e um status não é uma falha: `404` e `500` chegam como uma resposta comum com `ok` falso, porque a requisição alcançou um servidor e recebeu uma resposta. Uma falha de transporte retorna `Err(HttpError)` quando nenhuma resposta chega, com um `code` que identifica a causa: `ERR_HTTP_FETCH_DNS` quando o nome não resolve, `ERR_HTTP_FETCH_CONNECT` quando o host recusa, `ERR_HTTP_FETCH_TLS` quando o handshake falha, `ERR_HTTP_FETCH_TIMEOUT`, `ERR_HTTP_FETCH_REDIRECT`, `ERR_HTTP_FETCH_BODY_TOO_LARGE` e `ERR_HTTP_FETCH_URL`. Os três primeiros são separados porque pedem coisas diferentes: um nome que não resolve costuma ser erro de digitação ou configuração velha, uma conexão recusada muitas vezes é transitória e vale repetir, e um handshake que falha não se conserta repetindo. As conexões são reaproveitadas entre requisições ao mesmo host. O corpo é lido para a memória quando a requisição termina, limitado por `maxBodySize` (8 MB por padrão), então `text()` e `bytes()` são duas vistas de um corpo e não duas cópias dele; streaming é um contrato à parte. O `json<T>()` lê esse corpo pelo decodificador que um `T.fromJson(text)` usa, então uma resposta e uma string carregando o mesmo documento não podem discordar sobre o que ele diz, nem sobre qual campo está errado quando está: a falha é o mesmo `Result<T, JsonError>`, com o mesmo código e o mesmo caminho. As opções são escritas na chamada, porque `timeout` e `maxBodySize` são lidos em compilação, a mesma regra que as opções de servidor seguem.

O `fetch` decodifica respostas comprimidas sozinho. A menos que o programa defina `accept-encoding`, cada requisição oferece `gzip, deflate, br`, e um corpo de resposta numa dessas codificações chega decodificado, tanto em `fetch(...)` quanto em `fetch.stream(...)`, sem os headers `content-encoding` e `content-length`, que descrevem os bytes na rede. `maxBodySize` limita o corpo decodificado, um corpo que se expande mais de 200 vezes o tamanho comprimido depois do primeiro 1 MiB é recusado com `ERR_HTTP_FETCH_RATIO_LIMIT`, e um corpo que não confere com seu `Content-Encoding` é recusado com `ERR_HTTP_FETCH_DECODE`. Passe `decompress: false` para receber os bytes exatamente como o servidor os enviou, por exemplo para guardá-los comprimidos ou decodificá-los com [`tyrus:compress`](../compress/) sob limites próprios.

Use `fetch.stream(url, options?)` quando o corpo precisar ser lido progressivamente. Ele retorna `Ok(HttpClientStreamResponse)` depois que o cabeçalho da resposta chega ou `Err(HttpError)` se a requisição falhar antes, e `await response.stream()` devolve seu `ReadableStream<u8[]>` limitado; o contrato de `fetch(...)` comum continua em buffer.

Use `bodyStream` para enviar um corpo de requisição progressivo. Ele é separado do `body` textual, então a transferência fica visível no local da chamada; o transporte busca cada chunk apenas quando pode aceitá-lo, levando o backpressure até a origem. `fetch(...)` e todo verbo retornam `Promise<Result<HttpClientResponse, HttpError>>`.

Um corpo de requisição em stream só pode ser consumido uma vez e não pode ser reenviado. Se `fetch.stream(...)` ou `FetchOptions.bodyStream` receber um redirect, o cliente preserva a resposta 3xx e não faz uma segunda requisição; leia `status()` e `header("location")` se o programa quiser decidir o que fazer.

<!-- @include: ../../../.fragments/modules/http/index/exported-items.md -->

Os verbos penduram no próprio `fetch`: `fetch.get(url)`, `fetch.post(url, options)`, e o mesmo para `put`, `patch`, `delete`, `head`, `options`, `connect`, `query` e `trace`. Eles não são exportados sozinhos, porque `import { get, post }` traria esses nomes para o escopo de todo arquivo que quisesse um, e `get` é um nome que um programa tem usos próprios para. O verbo é a única forma de nomear um método: `FetchOptions` não tem campo `method`, então um `fetch(url, options)` simples envia `GET` e todo o resto é verbo. Escrever `method` mesmo assim é recusado com uma mensagem que nomeia o verbo a usar. Um programa é livre para nomear um valor seu de `fetch`, e aí os métodos são dele.

Toda resposta do servidor recebe um cabeçalho `x-request-id` com um ID de requisição Base64URL compacto sem padding. O access log vem desligado por padrão: passe `{ logger: true }` para `app.listen(...)` para escrever um evento estruturado em stderr sem dados da requisição. Quando desligado, o servidor não serializa nem escreve um evento de acesso. Registre `app.onResponse(...)` apenas quando o código da aplicação precisar de métricas próprias, registro de auditoria, integração de tracing ou outro trabalho após a resposta. O callback roda uma vez quando o servidor termina de produzir ou descarta o corpo, inclusive em um corpo em stream, e não pode alterar a resposta. Importe `Duration` de `tyrus:temporal` ao acessar `access.duration`.

## Início rápido

Uma REST API mínima com respostas tipadas e validação:

<!-- @include: ../../../.fragments/modules/http/index/quick-start.md -->

::: warning Mantenha camadas separadas
Regras de lint de API esperam que routes declarem roteamento, controllers traduzam fronteiras HTTP, services contenham comportamento de negócio e repositories concentrem acesso direto ao banco.
:::
