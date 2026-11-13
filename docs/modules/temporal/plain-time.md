---
title: PlainTime
description: Clock times without a date or time zone.
---

# PlainTime

`PlainTime` represents a clock time without a date or time zone. Addition wraps across midnight and preserves nanosecond precision.

## API

- `plainTimeFrom(value: string): Result<PlainTime, TemporalError>` parses ISO clock text.
- `plainTimeFromFields(fields: PlainTime): Result<PlainTime, TemporalError>` validates time fields.
- `plainTimeToString(value: PlainTime): Result<string, TemporalError>` formats the clock time.
- `plainTimeEquals(left, right): boolean` compares times by value.
- `plainTimeAddSeconds` and `plainTimeAddNanoseconds` return `Result<PlainTime, TemporalError>` after adding signed clock units with wrapping.
- `plainTimeAddDuration` and `plainTimeSubtractDuration` return `Result<PlainTime, TemporalError>` after applying a fixed clock `Duration`.
- `plainTimeUntil` and `plainTimeSince` return signed differences as `Result<Duration, TemporalError>`.
- `plainTimeUntilWithOptions` and `plainTimeSinceWithOptions` return `Result<Duration, TemporalError>` and accept `largestUnit`, `smallestUnit`, `roundingIncrement`, and `roundingMode`.
- `plainTimeSecondsUntil` and `plainTimeNanosecondsUntil` return scalar differences as `Result<i64, TemporalError>`.

<!-- @include: ../../.fragments/modules/temporal/plain-time/api.md -->
