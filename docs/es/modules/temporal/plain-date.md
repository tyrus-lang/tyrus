---
title: PlainDate
description: Fechas de calendario sin hora ni zona horaria.
---

# PlainDate

`PlainDate` representa una fecha de calendario sin hora ni zona horaria. Sus operaciones son inmutables.

## API

- `plainDateFrom(value: string): Result<PlainDate, TemporalError>` analiza una fecha ISO.
- `plainDateFromFields(fields: PlainDate): Result<PlainDate, TemporalError>` valida los campos de la fecha.
- `plainDateToString(value: PlainDate): Result<string, TemporalError>` formatea la fecha como `YYYY-MM-DD`.
- `plainDateEquals(left, right): boolean` compara fechas por valor.
- `plainDateAddDays`, `plainDateAddMonths` y `plainDateAddYears` devuelven `Result<PlainDate, TemporalError>` después de sumar unidades de calendario con signo.
- `plainDateAddDuration` y `plainDateSubtractDuration` devuelven `Result<PlainDate, TemporalError>` después de aplicar una `Duration`.
- `plainDateWith(value, fields): Result<PlainDate, TemporalError>` devuelve una fecha con campos reemplazados.
- `plainDateUntil` y `plainDateSince` devuelven diferencias como `Result<Duration, TemporalError>`.
- `plainDateUntilWithOptions` y `plainDateSinceWithOptions` aceptan `largestUnit`, `smallestUnit`, `roundingIncrement` y `roundingMode`.
- `plainDateDayOfWeek` y `plainDateDayOfYear` devuelven consultas del calendario en `Result<u32, TemporalError>`, y `plainDateDaysUntil` devuelve `Result<i64, TemporalError>`.

<!-- @include: ../../../.fragments/modules/temporal/plain-date/api.md -->
