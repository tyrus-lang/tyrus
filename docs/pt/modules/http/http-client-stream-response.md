---
title: HttpClientStreamResponse
description: Resposta do cliente HTTP cujo corpo é lido progressivamente.
---

# HttpClientStreamResponse

`HttpClientStreamResponse` é o valor de sucesso em `await fetch.stream(url, options?)`. A promise retorna `Err(HttpError)` se a requisição falhar antes da chegada do status e dos headers; depois disso, falhas do stream são informadas por `StreamError`.

<SignatureBlock label="Assinatura">HttpClientStreamResponse</SignatureBlock>

## Métodos

| Método | Assinatura | Finalidade |
| --- | --- | --- |
| `status` | `() => i32` | Devolve o código de status HTTP. |
| `ok` | `() => boolean` | Devolve se o status está na faixa `2xx`. |
| `header` | `(name: string) => string?` | Devolve um header da resposta sem considerar maiúsculas e minúsculas. |
| `stream` | `() => Promise<ReadableStream<u8[]>>` | Devolve o corpo legível progressivamente. |
| `close` | `() => void` | Aborta um corpo não lido e libera a resposta. |

## Uso

<!-- @include: ../../../.fragments/modules/http/http-client-stream-response/usage.md -->

Use `fetch(...)` comum quando o corpo inteiro for necessário como `text()`, `bytes()` ou `json<T>()`. `fetch.stream(...)` mantém no máximo um buffer limitado de transferência em memória e aplica backpressure enquanto `ReadableStream.read()` está esperando. Depois que `stream()` devolve seu leitor, `close()` libera o cabeçalho da resposta sem interromper esse leitor; feche o leitor para abandonar a transferência. Fechar uma resposta antes de obter um leitor cancela sua transferência. Uma falha de transporte antes de os headers chegarem retorna `Err(HttpError)` em `fetch.stream(...)`; uma falha depois desse ponto faz o stream legível falhar com `StreamError`.
