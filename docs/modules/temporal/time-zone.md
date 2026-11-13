---
title: TimeZone
description: The checked-in IANA time-zone enum and resolver.
---

# TimeZone

`TimeZone` is a closed enum generated from the checked-in IANA catalog. `timeZone` accepts an IANA identifier and validates that the runtime database provides it.

`timeZone(name: string): Result<TimeZone, TemporalError>` returns an `Err` for an unknown or unavailable IANA zone.

<!-- @include: ../../.fragments/modules/temporal/time-zone/timezone.md -->
