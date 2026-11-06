---
title: WritableStream
description: Envie uma sequência limitada de valores progressivamente.
---

# WritableStream

`WritableStream<T>` é uma capacidade somente de escrita devolvida por adaptadores nativos, como escrita de arquivo e entrada de processo filho.

<SignatureBlock label="Assinatura">WritableStream&lt;T&gt;</SignatureBlock>

## Métodos

| Método | Assinatura | Finalidade |
| --- | --- | --- |
| `write` | `(value: T) => Promise<void>` | Espera até a fila limitada aceitar um valor. |
| `close` | `() => Promise<void>` | Conclui escritas aceitas e fecha o lado gravável. |

## Uso

<!-- @include: ../../../.fragments/modules/stream/writable-stream.md -->

## Erros

`write()` rejeita com `StreamError` para um stream fechado, com falha ou usado concorrentemente, e rejeita imediatamente um chunk binário maior que a capacidade do buffer. `close()` é idempotente.
