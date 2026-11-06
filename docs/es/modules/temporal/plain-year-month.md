---
title: PlainYearMonth
description: Valores de año y mes del calendario.
---

# PlainYearMonth

`PlainYearMonth` representa un año y mes del calendario sin día, hora ni zona horaria.

## Construcción a partir de campos

`plainYearMonthFromFields(value: PlainYearMonth): Result<PlainYearMonth, TemporalError>` valida los campos `year` y `month` y devuelve el valor de año-mes validado. Úsala cuando se construya un valor de año-mes a partir de campos y sea necesario validar el calendario.

<!-- @include: ../../../.fragments/modules/temporal/plain-year-month/plainyearmonth.md -->
