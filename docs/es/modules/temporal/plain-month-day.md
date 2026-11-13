---
title: PlainMonthDay
description: Valores recurrentes de mes y día.
---

# PlainMonthDay

`PlainMonthDay` representa un mes y día recurrentes. La validación usa el año bisiesto 1972 para mantener representable el 29 de febrero.

## Construcción a partir de campos

`plainMonthDayFromFields(value: PlainMonthDay): Result<PlainMonthDay, TemporalError>` valida los campos `month` y `day` y devuelve el valor de mes-día validado. Usa el año bisiesto 1972, por lo que el 29 de febrero sigue siendo válido.

<!-- @include: ../../../.fragments/modules/temporal/plain-month-day/plainmonthday.md -->
