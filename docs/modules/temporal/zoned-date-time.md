---
title: ZonedDateTime
description: Absolute instants associated with IANA time zones.
---

# ZonedDateTime

`ZonedDateTime` combines an absolute instant with a checked-in IANA time zone. Parsing requires the bracketed IANA identifier and preserves it when formatting.

## API

- `zonedDateTimeFrom(value: string): Result<ZonedDateTime, TemporalError>` parses an ISO zoned date-time.
- `zonedDateTimeToInstant(value): Result<Instant, TemporalError>` removes the calendar projection.
- `zonedDateTimeToString(value): Result<string, TemporalError>` formats the value with its zone.
- `zonedDateTimeEquals(left, right): boolean` compares the complete zoned values.
- `zonedDateTimeAddDuration(value, duration): Result<ZonedDateTime, TemporalError>` applies a duration.
- `zonedDateTimeUntil(start, end): Result<Duration, TemporalError>` returns a signed difference.
- `zonedDateTimeFormat(value, pattern): Result<string, TemporalError>` supports `yyyy`, `MM`, `dd`, `HH`, `mm`, and `ss`.

<!-- @include: ../../.fragments/modules/temporal/zoned-date-time/api.md -->
