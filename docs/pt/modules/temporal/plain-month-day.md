---
title: PlainMonthDay
description: Valores recorrentes de mês e dia.
---

# PlainMonthDay

`PlainMonthDay` representa um mês e dia recorrentes. A validação usa o ano bissexto 1972 para manter o dia 29 de fevereiro representável.

## Construção a partir de campos

`plainMonthDayFromFields(value: PlainMonthDay): Result<PlainMonthDay, TemporalError>` valida os campos `month` e `day` e retorna o valor de mês-dia validado. Usa o ano bissexto 1972, então 29 de fevereiro continua válido.

<!-- @include: ../../../.fragments/modules/temporal/plain-month-day/plainmonthday.md -->
