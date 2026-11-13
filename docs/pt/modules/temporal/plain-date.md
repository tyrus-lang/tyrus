---
title: PlainDate
description: Datas de calendário sem hora ou fuso horário.
---

# PlainDate

`PlainDate` representa uma data de calendário sem hora ou fuso horário. Suas operações são imutáveis.

## API

- `plainDateFrom(value: string): Result<PlainDate, TemporalError>` analisa uma data ISO.
- `plainDateFromFields(fields: PlainDate): Result<PlainDate, TemporalError>` valida os campos da data.
- `plainDateToString(value: PlainDate): Result<string, TemporalError>` formata a data como `YYYY-MM-DD`.
- `plainDateEquals(left, right): boolean` compara datas por valor.
- `plainDateAddDays`, `plainDateAddMonths` e `plainDateAddYears` retornam `Result<PlainDate, TemporalError>` após adicionar unidades de calendário com sinal.
- `plainDateAddDuration` e `plainDateSubtractDuration` retornam `Result<PlainDate, TemporalError>` após aplicar uma `Duration`.
- `plainDateWith(value, fields): Result<PlainDate, TemporalError>` retorna uma data com campos substituídos.
- `plainDateUntil` e `plainDateSince` retornam diferenças em `Result<Duration, TemporalError>`.
- `plainDateUntilWithOptions` e `plainDateSinceWithOptions` aceitam `largestUnit`, `smallestUnit`, `roundingIncrement` e `roundingMode`.
- `plainDateDayOfWeek` e `plainDateDayOfYear` retornam consultas do calendário em `Result<u32, TemporalError>`, e `plainDateDaysUntil` retorna `Result<i64, TemporalError>`.

<!-- @include: ../../../.fragments/modules/temporal/plain-date/api.md -->
