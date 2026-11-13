---
title: Instant
description: Puntos absolutos en la línea temporal de tyrus.
---

# Instant

`Instant` representa un punto absoluto en la línea temporal con segundos con signo y un resto de nanosegundos no negativo.

Cuando Tyrus serializa un `Instant` como JSON, usa la cadena ISO 8601 en UTC devuelta por `instantToString`, también en respuestas HTTP. OpenAPI describe este valor como `type: "string"` con `format: "date-time"`, igual que valores como `2026-09-05T18:32:52.54345Z`.

## API

- `instantNow(): Instant` lee el reloj nativo actual.
- `instantFrom(value: string): Result<Instant, TemporalError>` analiza un instante ISO.
- `instantFromEpochMilliseconds(milliseconds: i64): Result<Instant, TemporalError>` crea un instante a partir de milisegundos desde Unix Epoch.
- `instantToEpochMilliseconds(value: Instant): Result<i64, TemporalError>` convierte un instante a milisegundos desde el epoch.
- `instantToString(value: Instant): Result<string, TemporalError>` formatea un instante como texto ISO UTC.
- `instantAddDuration` y `instantSubtractDuration` devuelven `Result<Instant, TemporalError>` y aplican duraciones de tamaño fijo; las unidades de calendario se rechazan.
- `instantToZonedDateTime(value: Instant, timeZone: TimeZone): ZonedDateTime` asocia un instante con una zona IANA versionada en el repositorio.

<!-- @include: ../../../.fragments/modules/temporal/instant/api.md -->
