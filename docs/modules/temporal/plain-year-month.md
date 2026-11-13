---
title: PlainYearMonth
description: Calendar year and month values.
---

# PlainYearMonth

`PlainYearMonth` represents a calendar year and month without a day, time, or time zone.

## Constructing from fields

`plainYearMonthFromFields(value: PlainYearMonth): Result<PlainYearMonth, TemporalError>` validates the `year` and `month` fields and returns the validated year-month value. Use it when a year-month value is assembled from fields and needs calendar validation.

<!-- @include: ../../.fragments/modules/temporal/plain-year-month/plainyearmonth.md -->
