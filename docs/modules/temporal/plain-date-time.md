---
title: PlainDateTime
description: Calendar dates combined with clock times.
---

# PlainDateTime

`PlainDateTime` composes a `PlainDate` and a `PlainTime` without a time zone. Date arithmetic carries across midnight and preserves the civil calendar.

## API

- `plainDateTimeFrom(value: string): Result<PlainDateTime, TemporalError>` parses ISO date-time text.
- `plainDateTimeFromFields(fields: PlainDateTime): Result<PlainDateTime, TemporalError>` validates composed fields.
- `plainDateTimeToString(value): Result<string, TemporalError>` formats ISO text with `T`.
- `plainDateTimeEquals` compares values, while `plainDateTimeWith`, `plainDateTimeAddDuration`, and `plainDateTimeSubtractDuration` return `Result<PlainDateTime, TemporalError>`.
- `plainDateTimeUntil` and `plainDateTimeSince` return signed durations as `Result<Duration, TemporalError>`.
- `plainDateTimeUntilWithOptions` and `plainDateTimeSinceWithOptions` return `Result<Duration, TemporalError>` and accept `largestUnit`, `smallestUnit`, `roundingIncrement`, and `roundingMode`.

<!-- @include: ../../.fragments/modules/temporal/plain-date-time/api.md -->
