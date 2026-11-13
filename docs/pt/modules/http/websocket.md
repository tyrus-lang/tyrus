---
title: WebSocket
description: Rotas e clientes WebSocket com mensagens inteiras e backpressure.
---

# WebSocket

Um WebSocket é uma conexão bidirecional de longa duração. `app.ws(path, options)` transforma uma rota em uma, e `connectWebSocket(url, protocols?)` retorna `Promise<Result<WebSocket, HttpError>>`. As duas pontas recebem o mesmo valor `WebSocket`, então ler e escrever é igual dos dois lados.

## Opções da rota

| Opção | Tipo | Finalidade |
| --- | --- | --- |
| `handler` | `(socket: WebSocket) => Promise<void>` | Obrigatório. Roda uma vez por conexão; a conexão fecha quando sua Promise termina. |
| `origins` | `string[]` | Origens de navegador que podem conectar. Sem ela só a própria origem do servidor é aceita; uma requisição sem header `Origin`, que não vem de uma página de navegador, é sempre aceita. |
| `subprotocols` | `string[]` | Subprotocolos que a rota fala, em ordem de preferência. |
| `maxMessageBytes` | `i64` | Maior mensagem aceita, 1 MB por padrão. Uma mensagem maior faz falhar o `receive` que a lê. |
| `preHandler` | `((request: HttpRequest) => HttpResponse \| true)[]` | Funções nomeadas executadas sobre a requisição de upgrade antes de aceitá-la, onde cookies e headers decidem quem pode conectar; a primeira que retornar uma resposta recusa o upgrade com ela. Upgrades também contam no `rateLimit` do servidor. |
| `config` | `{ rateLimit?: RateLimit }` | Um limite de upgrades para esta rota, escrito como o `config.rateLimit` de uma rota HTTP; ele substitui o `rateLimit` do servidor para esta rota. |

## Métodos

| Método | Assinatura | Finalidade |
| --- | --- | --- |
| `stream` | `() => Promise<Result<DuplexStream<u8[], u8[]>, HttpError>>` | Transfere a I/O de mensagens binárias para um duplex limitado. Cada chunk é uma mensagem WebSocket binária completa. |
| `receive` | `() => Promise<Result<WebSocketMessage?, HttpError>>` | Espera a próxima mensagem inteira; resolve sem mensagem quando o outro lado fechou a conexão. |
| `sendText` | `(text: string) => Promise<Result<void, HttpError>>` | Envia uma mensagem de texto e resolve quando ela foi escrita. |
| `sendBytes` | `(data: u8[]) => Promise<Result<void, HttpError>>` | Envia uma mensagem binária e resolve quando ela foi escrita. |
| `close` | `(code?: i32, reason?: string) => Promise<Result<void, HttpError>>` | Envia um fechamento com `code` (1000 por padrão, ou de 3000 a 4999) e um `reason` opcional de até 123 bytes, e encerra a conexão; fechar duas vezes não é erro. |
| `protocol` | `() => string` | O subprotocolo combinado, ou uma string vazia. |
| `closeCode` | `() => i32?` | O código de fechamento recebido do outro lado depois que `receive()` observa seu frame de fechamento, ou sem valor antes disso ou após um fim de transporte abrupto. |

`WebSocketMessage` carrega uma mensagem inteira, nunca um pedaço dela: `binary` diz qual campo a contém, `text` para uma mensagem de texto e `bytes` para uma binária. Pings são respondidos automaticamente e nunca chegam ao programa. O runtime também envia um ping de heartbeat a cada 30 segundos; se o outro lado não responder à sonda seguinte, o socket termina e um `receive()` pendente resolve sem mensagem.

`stream()` transfere a I/O de mensagens binárias para um `DuplexStream<u8[], u8[]>`. Seus chunks legíveis e graváveis são cada qual uma mensagem WebSocket binária completa, nunca um fragmento. A transferência faz `receive()`, `sendText()` e `sendBytes()` retornarem `Err(HttpError)` com `ERR_HTTP_WS_STREAMED`, para que não disputem o duplex. Fechar seu lado gravável inicia o handshake de fechamento enquanto o lado legível ainda pode receber as mensagens finais do outro lado. Uma mensagem de texto recebida não pode virar bytes silenciosamente: ela falha o lado legível com o código `ERR_STREAM_WS_TEXT_MESSAGE` de `StreamError`.

Um envio retorna `Promise<Result<void, HttpError>>` e espera a conexão aceitar a mensagem, então um remetente rápido desacelera até o ritmo de um destinatário lento em vez de enfileirar sem limite. Só um `receive` pode ficar pendente por vez. `server.close()` fecha todo socket aberto com o código 1001 e espera o handler dele, dentro de `shutdownTimeout`; um `receive` pendente no handler resolve sem mensagem.

## Erros

Todos os métodos falíveis de WebSocket retornam falhas recuperáveis como `Err(HttpError)`. Os códigos incluem `ERR_HTTP_WS_CLOSED`, `ERR_HTTP_WS_STREAMED`, `ERR_HTTP_WS_BUSY`, `ERR_HTTP_WS_SEND`, `ERR_HTTP_WS_CLOSE_CODE` e `ERR_HTTP_WS_CLOSE_REASON`; erros de recebimento incluem `ERR_HTTP_WS_MESSAGE_TOO_LARGE` e `ERR_HTTP_WS_RECEIVE`. Falhas de conexão retornam `Err(HttpError)` por `connectWebSocket`, com os códigos `ERR_HTTP_WS_URL`, `ERR_HTTP_WS_DNS`, `ERR_HTTP_WS_CONNECT`, `ERR_HTTP_WS_TLS`, `ERR_HTTP_WS_HANDSHAKE` ou `ERR_HTTP_FETCH_BLOCKED` quando `app.fetchPolicy(...)` recusa o destino, que ela julga do mesmo jeito que julga `fetch`.

## Uso

<!-- @include: ../../../.fragments/modules/http/websocket/usage.md -->
