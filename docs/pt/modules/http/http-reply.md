---
title: HttpReply
description: API de reply HTTP tipada para controllers tyrus.
---

# HttpReply

`HttpReply<T>` é o objeto de response tipado usado por controllers HTTP.

Declare os corpos de erro da aplicação com os campos obrigatórios `code: string` e `message: string`. O handler deve enviar os dois campos, por exemplo `{ code: "user_not_found", message: "User not found" }`; o Tyrus não deriva nem sobrescreve `code` a partir do status HTTP. O OpenAPI documenta os campos declarados pela aplicação.

Controllers podem retornar `void` ou `Promise<void>`. Controllers assíncronos inline inferem `Promise<void>` quando a anotação de retorno é omitida, e o runtime HTTP aguarda a conclusão antes de finalizar a resposta. As verificações de envio único e de todos os caminhos de sucesso incluem código nos dois lados de `await`.

## Uso

<!-- @include: ../../../.fragments/modules/http/http-reply/usage.md -->
Os schemas de resposta de erro pertencem à aplicação. Um service deve retornar dados de domínio ou lançar um erro de domínio; um controller ou futura fronteira de erros o mapeia para um status HTTP e este corpo de resposta declarado.

## Headers e cookies

Prepare os metadados da resposta antes de `status(...).send(...)`. `header(name, value)` substitui um header, `appendHeader(name, value)` adiciona outro valor, `removeHeader(name)` remove valores definidos antes no reply e `type(value)` define `Content-Type`.

<!-- @include: ../../../.fragments/modules/http/http-reply/headers-and-cookies.md -->

Nomes e valores de headers são validados na fronteira nativa. Headers que controlam o framing da mensagem (`connection`, `content-length` e `transfer-encoding`) e `set-cookie` não podem ser alterados pela API genérica de headers. Use `setCookie(name, value, options?)` e `clearCookie` para cookies. `setCookies(cookies)` define de uma vez uma lista de valores `HttpCookie` (`name`, `value`, `options?`) e define todos ou nenhum: quando qualquer entrada é rejeitada, nenhum cookie da lista é escrito. `clearCookies()` expira todos os cookies que o request enviou; o servidor só conhece esses, então um request sem cookies deixa a resposta inalterada. As opções de cookie podem definir `domain`, `path`, `maxAge`, `secure`, `httpOnly` e `sameSite`; `sameSite: "none"` exige `secure: true`. JSON é o tipo de conteúdo padrão quando `type(...)` não é chamado.

Um `send` ou `stream` precisa ser precedido por `status(...)`: o slot de resposta começa em `500`, então enviar sem escolher um status serviria o corpo de sucesso sob um status de falha. O checker reporta isso no método final da resposta. `stream()` está disponível apenas para um status declarado como `StreamBody`; ele retorna um `WritableStream<u8[]>` que inicia a resposta progressivamente. Consulte [`StreamBody`](./stream-body.md) para a assinatura e o ciclo de vida.

Nomes e valores de headers e cookies são validados. Um nome escrito como literal é verificado em compilação, com o erro apontando para o literal; um nome construído em tempo de execução é verificado na fronteira nativa, onde a rejeição é escrita em stderr e o header ou cookie não é enviado. Headers de framing (`connection`, `content-length`, `transfer-encoding`) e `set-cookie` são protegidos: use os métodos de cookie para cookies.

## Cache condicional

Passe metadados de cache como o segundo argumento opcional de `send`. `HttpCacheOptions.etag` é o valor opaco do validador; o Tyrus coloca aspas e o valida. Defina `weak: true` somente quando o validador for intencionalmente fraco. `lastModified` aceita um `Instant` e é formatado como data HTTP, enquanto `control` se torna `Cache-Control`. Quando `If-None-Match` contém a mesma tag, incluindo sua forma fraca ou uma tag em uma lista separada por vírgulas, o Tyrus retorna `304 Not Modified`, preserva o ETag e omite o corpo.

Use uma data HTTP válida em `last-modified` quando a representação tiver um instante de modificação significativo. Na ausência de `If-None-Match`, `If-Modified-Since` igual ou posterior a esse instante também produz `304 Not Modified`. `If-None-Match` sempre tem precedência.

## Compressão de resposta

O Tyrus negocia `br` ou `gzip` automaticamente para respostas bufferizadas, compressíveis e com pelo menos 1024 bytes quando a requisição os aceita. Brotli vence quando ambos têm a mesma qualidade. Uma resposta comprimida inclui seu `Content-Encoding` e `Vary: Accept-Encoding`. Ele nunca recomprime um `Content-Encoding` explícito, formatos já compactados ou uma resposta com ETag forte, pois um validador forte identifica uma representação exata. Compressão por streaming é planejada separadamente.

<!-- @include: ../../../.fragments/modules/http/http-reply/response-compression.md -->

<!-- @include: ../../../.fragments/modules/http/http-reply/response-compression-2.md -->

`controller-must-send-response` valida que caminhos de sucesso do controller enviem uma resposta.
