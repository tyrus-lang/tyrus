---
title: HttpError
description: Construtores tipados de erro HTTP de tyrus:http.
---

# HttpError

`HttpError` fornece construtores tipados para falhas HTTP esperadas.

## Construtores

| Construtor | Assinatura |
|------------|------------|
| `BadRequest` | `(message: string) => HttpError` |
| `Unauthorized` | `(message: string) => HttpError` |
| `Forbidden` | `(message: string) => HttpError` |
| `NotFound` | `(message: string) => HttpError` |
| `Conflict` | `(message: string) => HttpError` |
| `UnprocessableEntity` | `(message: string) => HttpError` |
| `InternalServerError` | `(message: string) => HttpError` |
| `custom` | `(status: HttpStatus, errorCode: string, message: string) => HttpError` |

<SignatureBlock label="Assinatura">const error = HttpError.custom(HttpStatus.TooManyRequests, "rate_limit", "Too many requests");</SignatureBlock>


<!-- @include: ../../../.fragments/modules/http/http-error/constructors.md -->
