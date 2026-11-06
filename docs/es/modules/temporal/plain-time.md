---
title: PlainTime
description: Horas sin fecha ni zona horaria.
---

# PlainTime

`PlainTime` representa una hora sin fecha ni zona horaria. La suma atraviesa la medianoche y conserva la precisión de nanosegundos.

## API

- `plainTimeFrom(value: string): Result<PlainTime, TemporalError>` analiza un texto de reloj ISO.
- `plainTimeFromFields(fields: PlainTime): Result<PlainTime, TemporalError>` valida los campos de la hora.
- `plainTimeToString(value: PlainTime): Result<string, TemporalError>` formatea la hora.
- `plainTimeEquals(left, right): boolean` compara horas por valor.
- `plainTimeAddSeconds` y `plainTimeAddNanoseconds` devuelven `Result<PlainTime, TemporalError>` después de sumar unidades con signo y envolver el reloj.
- `plainTimeAddDuration` y `plainTimeSubtractDuration` devuelven `Result<PlainTime, TemporalError>` después de aplicar una `Duration` de reloj.
- `plainTimeUntil` y `plainTimeSince` devuelven diferencias como `Result<Duration, TemporalError>`.
- `plainTimeUntilWithOptions` y `plainTimeSinceWithOptions` devuelven `Result<Duration, TemporalError>` y aceptan `largestUnit`, `smallestUnit`, `roundingIncrement` y `roundingMode`.
- `plainTimeSecondsUntil` y `plainTimeNanosecondsUntil` devuelven diferencias escalares como `Result<i64, TemporalError>`.

<!-- @include: ../../../.fragments/modules/temporal/plain-time/api.md -->
