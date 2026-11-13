---
title: PlainYearMonth
description: Valores de ano e mês do calendário.
---

# PlainYearMonth

`PlainYearMonth` representa um ano e mês do calendário sem dia, hora ou fuso horário.

## Construção a partir de campos

`plainYearMonthFromFields(value: PlainYearMonth): Result<PlainYearMonth, TemporalError>` valida os campos `year` e `month` e retorna o valor de ano-mês validado. Use-a quando um valor de ano-mês for montado a partir de campos e precisar de validação de calendário.

<!-- @include: ../../../.fragments/modules/temporal/plain-year-month/plainyearmonth.md -->
