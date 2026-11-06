---
title: Temporal
description: Valores nativos de fecha y hora expuestos por tyrus:temporal.
---

# Temporal

`tyrus:temporal` proporciona valores nativos para instantes absolutos, fechas de calendario, horas, duraciones, fechas y horas con zona y zonas horarias IANA versionadas en el repositorio. Todas las API nativas falibles del módulo devuelven los fallos recuperables como `Result<T, TemporalError>`; las operaciones sin fallos, como la igualdad de valores, devuelven directamente el valor.

## APIs disponibles

| API | Propósito |
|-----|----------|
| [`Instant`](./instant.md) | Puntos absolutos en la línea temporal. |
| [`PlainDate`](./plain-date.md) | Fechas de calendario sin hora ni zona horaria. |
| [`PlainTime`](./plain-time.md) | Horas sin fecha ni zona horaria. |
| [`PlainDateTime`](./plain-date-time.md) | Fechas de calendario combinadas con horas. |
| [`ZonedDateTime`](./zoned-date-time.md) | Instantes asociados a una zona horaria IANA. |
| [`Duration`](./duration.md) | Unidades de calendario y reloj con signo. |
| [`TimeZone`](./time-zone.md) | El enum IANA versionado en el repositorio y su resolvedor. |
| [`PlainYearMonth`](./plain-year-month.md) | Valores de año y mes del calendario. |
| [`PlainMonthDay`](./plain-month-day.md) | Valores recurrentes de mes y día. |

## Importación

<SignatureBlock label="Importaciones">import { Instant, PlainDate, TimeZone, instantNow, plainDateFrom, timeZone } from "tyrus:temporal";</SignatureBlock>
