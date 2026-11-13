---
title: Temporal
description: Native date and time values exposed by tyrus:temporal.
---

# Temporal

`tyrus:temporal` provides native values for absolute instants, calendar dates, clock times, durations, zoned date-times, and checked-in IANA time zones. Every fallible native API in the module reports recoverable failures as `Result<T, TemporalError>`; non-fallible operations such as value equality return their direct value.

## Available APIs

| API | Purpose |
|-----|---------|
| [`Instant`](./instant.md) | Absolute points on the timeline. |
| [`PlainDate`](./plain-date.md) | Calendar dates without a time or time zone. |
| [`PlainTime`](./plain-time.md) | Clock times without a date or time zone. |
| [`PlainDateTime`](./plain-date-time.md) | Calendar dates combined with clock times. |
| [`ZonedDateTime`](./zoned-date-time.md) | Instants associated with an IANA time zone. |
| [`Duration`](./duration.md) | Signed calendar and clock units. |
| [`TimeZone`](./time-zone.md) | The checked-in IANA time-zone enum and resolver. |
| [`PlainYearMonth`](./plain-year-month.md) | Calendar year and month values. |
| [`PlainMonthDay`](./plain-month-day.md) | Recurring month and day values. |

## Import

<SignatureBlock label="Imports">import { Instant, PlainDate, TimeZone, instantNow, plainDateFrom, timeZone } from "tyrus:temporal";</SignatureBlock>
