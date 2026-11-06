---
title: ZonedDateTime
description: Instantes absolutos asociados con zonas horarias IANA.
---

# ZonedDateTime

`ZonedDateTime` combina un instante absoluto con una zona horaria IANA versionada en el repositorio. El parsing exige el identificador IANA entre corchetes y lo conserva al formatear.

## API

- `zonedDateTimeFrom(value: string): Result<ZonedDateTime, TemporalError>` analiza una fecha y hora ISO con zona.
- `zonedDateTimeToInstant(value): Result<Instant, TemporalError>` elimina la proyección de calendario.
- `zonedDateTimeToString(value): Result<string, TemporalError>` formatea el valor con su zona.
- `zonedDateTimeEquals(left, right): boolean` compara los valores zonificados completos.
- `zonedDateTimeAddDuration(value, duration): Result<ZonedDateTime, TemporalError>` aplica una duración.
- `zonedDateTimeUntil(start, end): Result<Duration, TemporalError>` devuelve una diferencia con signo.
- `zonedDateTimeFormat(value, pattern): Result<string, TemporalError>` acepta `yyyy`, `MM`, `dd`, `HH`, `mm` y `ss`.

<!-- @include: ../../../.fragments/modules/temporal/zoned-date-time/api.md -->
