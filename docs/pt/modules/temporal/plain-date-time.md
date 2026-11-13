---
title: PlainDateTime
description: Datas de calendário combinadas com horários.
---

# PlainDateTime

`PlainDateTime` combina um `PlainDate` e um `PlainTime` sem fuso horário. A aritmética atravessa a meia-noite e preserva o calendário civil.

## API

- `plainDateTimeFrom(value: string): Result<PlainDateTime, TemporalError>` analisa texto de data e hora ISO.
- `plainDateTimeFromFields(fields: PlainDateTime): Result<PlainDateTime, TemporalError>` valida campos compostos.
- `plainDateTimeToString(value): Result<string, TemporalError>` formata texto ISO com `T`.
- `plainDateTimeEquals` compara valores, enquanto `plainDateTimeWith`, `plainDateTimeAddDuration` e `plainDateTimeSubtractDuration` retornam `Result<PlainDateTime, TemporalError>`.
- `plainDateTimeUntil` e `plainDateTimeSince` retornam durações com sinal em `Result<Duration, TemporalError>`.
- `plainDateTimeUntilWithOptions` e `plainDateTimeSinceWithOptions` retornam `Result<Duration, TemporalError>` e aceitam `largestUnit`, `smallestUnit`, `roundingIncrement` e `roundingMode`.

<!-- @include: ../../../.fragments/modules/temporal/plain-date-time/api.md -->
