---
title: PlainDate
description: Calendar dates without a time or time zone.
---

# PlainDate

`PlainDate` represents a calendar date without a time or time zone. Its operations are immutable.

## API

- `plainDateFrom(value: string): Result<PlainDate, TemporalError>` parses an ISO date.
- `plainDateFromFields(fields: PlainDate): Result<PlainDate, TemporalError>` validates date fields.
- `plainDateToString(value: PlainDate): Result<string, TemporalError>` formats the date as `YYYY-MM-DD`.
- `plainDateEquals(left, right): boolean` compares dates by value.
- `plainDateAddDays`, `plainDateAddMonths`, and `plainDateAddYears` return `Result<PlainDate, TemporalError>` after adding signed calendar units.
- `plainDateAddDuration` and `plainDateSubtractDuration` return `Result<PlainDate, TemporalError>` after applying a `Duration`.
- `plainDateWith(value, fields): Result<PlainDate, TemporalError>` returns a date with replacement fields.
- `plainDateUntil` and `plainDateSince` return `Result<Duration, TemporalError>` differences.
- `plainDateUntilWithOptions` and `plainDateSinceWithOptions` accept `largestUnit`, `smallestUnit`, `roundingIncrement`, and `roundingMode`.
- `plainDateDayOfWeek` and `plainDateDayOfYear` return `Result<u32, TemporalError>` calendar queries, and `plainDateDaysUntil` returns `Result<i64, TemporalError>`.

<!-- @include: ../../.fragments/modules/temporal/plain-date/api.md -->
