---
title: Duration
description: Unidades de calendario y reloj con signo.
---

# Duration

`Duration` almacena unidades de calendario y reloj con signo. Usa `durationTotal` solo con unidades de longitud fija; los años y meses requieren una referencia de calendario.

## API

- `durationFromFields(fields: Duration): Duration` valida y devuelve los campos de la duración.
- `durationTotal(value, unit): Result<i64, TemporalError>` convierte unidades de longitud fija, como segundos, minutos, horas y días.
- `plainDateAddDuration`, `plainDateSubtractDuration`, `plainTimeAddDuration`, `plainTimeSubtractDuration` y `zonedDateTimeAddDuration` aplican duraciones a valores temporales.
- `plainDateUntil`, `plainDateSince`, `plainTimeUntil`, `plainTimeSince` y `zonedDateTimeUntil` producen duraciones.
- `durationRound(value, unit, increment, mode, relativeTo): Result<Duration, TemporalError>` redondea una duración; las unidades de calendario requieren la fecha explícita `relativeTo`.

<!-- @include: ../../../.fragments/modules/temporal/duration/api.md -->
