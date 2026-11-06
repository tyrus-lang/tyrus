---
title: HttpError
description: Typed HTTP error constructors from tyrus:http.
---

# HttpError

`HttpError` provides typed constructors for expected HTTP failures.

## Constructors

| Constructor | Signature |
|-------------|-----------|
| `BadRequest` | `(message: string) => HttpError` |
| `Unauthorized` | `(message: string) => HttpError` |
| `Forbidden` | `(message: string) => HttpError` |
| `NotFound` | `(message: string) => HttpError` |
| `Conflict` | `(message: string) => HttpError` |
| `UnprocessableEntity` | `(message: string) => HttpError` |
| `InternalServerError` | `(message: string) => HttpError` |
| `custom` | `(status: HttpStatus, errorCode: string, message: string) => HttpError` |

<SignatureBlock label="Signature">const error = HttpError.custom(HttpStatus.TooManyRequests, "rate_limit", "Too many requests");</SignatureBlock>


<!-- @include: ../../.fragments/modules/http/http-error/constructors.md -->
