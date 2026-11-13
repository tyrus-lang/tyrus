---
title: PlainMonthDay
description: Recurring month and day values.
---

# PlainMonthDay

`PlainMonthDay` represents a recurring month and day. Validation uses leap year 1972 so February 29 remains representable.

## Constructing from fields

`plainMonthDayFromFields(value: PlainMonthDay): Result<PlainMonthDay, TemporalError>` validates the `month` and `day` fields and returns the validated month-day value. It uses leap year 1972, so February 29 remains valid.

<!-- @include: ../../.fragments/modules/temporal/plain-month-day/plainmonthday.md -->
