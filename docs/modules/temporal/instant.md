---
title: Instant
description: Absolute points on the tyrus timeline.
---

# Instant

`Instant` represents an absolute point on the timeline with signed seconds and a non-negative nanosecond remainder.

When Tyrus serializes an `Instant` as JSON, it uses the UTC ISO 8601 string returned by `instantToString`, including it as a string field in HTTP responses. OpenAPI describes this value as `type: "string"` with `format: "date-time"`, matching values such as `2026-09-05T18:32:52.54345Z`.

## API

- `instantNow(): Instant` reads the current native clock.
- `instantFrom(value: string): Result<Instant, TemporalError>` parses an ISO instant.
- `instantFromEpochMilliseconds(milliseconds: i64): Result<Instant, TemporalError>` creates an instant from Unix epoch milliseconds.
- `instantToEpochMilliseconds(value: Instant): Result<i64, TemporalError>` converts an instant to epoch milliseconds.
- `instantToString(value: Instant): Result<string, TemporalError>` formats an instant as UTC ISO text.
- `instantAddDuration` and `instantSubtractDuration` return `Result<Instant, TemporalError>` and apply fixed-length durations; calendar units are rejected.
- `instantToZonedDateTime(value: Instant, timeZone: TimeZone): ZonedDateTime` associates an instant with a checked-in IANA zone.

<!-- @include: ../../.fragments/modules/temporal/instant/api.md -->
