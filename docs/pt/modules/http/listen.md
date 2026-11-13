---
title: listen
description: Inicie um servidor HTTP e trate falhas de inicialização.
---

# listen

`app.listen(port, options?)` associa o roteador e retorna `Result<HttpServer, HttpError>`. O servidor começa a aceitar requisições imediatamente, e `main` pode continuar executando o código de inicialização. Use `onfail` para tratar uma falha de bind ou de inicialização TLS; em uma função `void`, o bloco de falha retorna dessa função.

<SignatureBlock label="Assinatura">app.listen(port: i32, options?: ServerOptions): Result&lt;HttpServer, HttpError&gt;</SignatureBlock>

## Exibir a porta vinculada

`listen` retorna o handle do servidor vinculado quando tem sucesso, então `server.port()` informa a porta real, inclusive quando a porta solicitada foi `0`. O runtime mantém o listener ativo depois que `main` retorna e aguarda um sinal de encerramento do sistema operacional; chame `server.close()` quando a aplicação precisar pará-lo antes.

<!-- @include: ../../../.fragments/modules/http/listen-example.md -->

## Tratar uma falha de inicialização

A variável de `onfail` contém `HttpError`, com um `code` estável e uma `message` legível. O erro é tratado sem lançar uma exceção; o bloco retorna dessa função `void` depois de informá-lo.

<!-- @include: ../../../.fragments/modules/http/listen-catch.md -->

## Opções

Passe um objeto `ServerOptions` como segundo argumento de `listen`. Consulte [ServerOptions](./server-options.md) para ver as configurações disponíveis.
