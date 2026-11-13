---
title: HttpError
description: Constructores tipados de error HTTP de tyrus:http.
---

# HttpError

`HttpError` proporciona constructores tipados para fallas HTTP esperadas.

## Constructores

| Constructor | Firma |
|-------------|-------|
| `BadRequest` | `(message: string) => HttpError` |
| `Unauthorized` | `(message: string) => HttpError` |
| `Forbidden` | `(message: string) => HttpError` |
| `NotFound` | `(message: string) => HttpError` |
| `Conflict` | `(message: string) => HttpError` |
| `UnprocessableEntity` | `(message: string) => HttpError` |
| `InternalServerError` | `(message: string) => HttpError` |
| `custom` | `(status: HttpStatus, errorCode: string, message: string) => HttpError` |

<SignatureBlock label="Firma">const error = HttpError.custom(HttpStatus.TooManyRequests, "rate_limit", "Too many requests");</SignatureBlock>


<!-- @include: ../../../.fragments/modules/http/http-error/constructors.md -->
