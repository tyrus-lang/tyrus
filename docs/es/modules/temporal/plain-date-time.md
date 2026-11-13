---
title: PlainDateTime
description: Fechas de calendario combinadas con horas.
---

# PlainDateTime

`PlainDateTime` combina un `PlainDate` y un `PlainTime` sin zona horaria. La aritmética atraviesa la medianoche y conserva el calendario civil.

## API

- `plainDateTimeFrom(value: string): Result<PlainDateTime, TemporalError>` analiza texto de fecha y hora ISO.
- `plainDateTimeFromFields(fields: PlainDateTime): Result<PlainDateTime, TemporalError>` valida campos compuestos.
- `plainDateTimeToString(value): Result<string, TemporalError>` formatea texto ISO con `T`.
- `plainDateTimeEquals` compara valores, mientras que `plainDateTimeWith`, `plainDateTimeAddDuration` y `plainDateTimeSubtractDuration` devuelven `Result<PlainDateTime, TemporalError>`.
- `plainDateTimeUntil` y `plainDateTimeSince` devuelven duraciones con signo como `Result<Duration, TemporalError>`.
- `plainDateTimeUntilWithOptions` y `plainDateTimeSinceWithOptions` devuelven `Result<Duration, TemporalError>` y aceptan `largestUnit`, `smallestUnit`, `roundingIncrement` y `roundingMode`.

<!-- @include: ../../../.fragments/modules/temporal/plain-date-time/api.md -->
