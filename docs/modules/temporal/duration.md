---
title: Duration
description: Signed calendar and clock units.
---

# Duration

`Duration` stores signed calendar and clock units. Use `durationTotal` only with fixed-length units; years and months require a calendar reference.

## API

- `durationFromFields(fields: Duration): Duration` validates and returns duration fields.
- `durationTotal(value, unit): Result<i64, TemporalError>` converts fixed-length units such as seconds, minutes, hours, and days.
- `plainDateAddDuration`, `plainDateSubtractDuration`, `plainTimeAddDuration`, `plainTimeSubtractDuration`, and `zonedDateTimeAddDuration` apply durations to temporal values.
- `plainDateUntil`, `plainDateSince`, `plainTimeUntil`, `plainTimeSince`, and `zonedDateTimeUntil` produce durations.
- `durationRound(value, unit, increment, mode, relativeTo): Result<Duration, TemporalError>` rounds a duration; calendar units require the explicit `relativeTo` date.

<!-- @include: ../../.fragments/modules/temporal/duration/api.md -->
