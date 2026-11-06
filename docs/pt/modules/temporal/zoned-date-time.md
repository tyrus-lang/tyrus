---
title: ZonedDateTime
description: Instantes absolutos associados a fusos horários IANA.
---

# ZonedDateTime

`ZonedDateTime` combina um instante absoluto com um fuso horário IANA versionado no repositório. O parsing exige o identificador IANA entre colchetes e o preserva ao formatar.

## API

- `zonedDateTimeFrom(value: string): Result<ZonedDateTime, TemporalError>` analisa uma data e hora ISO com fuso.
- `zonedDateTimeToInstant(value): Result<Instant, TemporalError>` remove a projeção de calendário.
- `zonedDateTimeToString(value): Result<string, TemporalError>` formata o valor com seu fuso.
- `zonedDateTimeEquals(left, right): boolean` compara os valores zonados completos.
- `zonedDateTimeAddDuration(value, duration): Result<ZonedDateTime, TemporalError>` aplica uma duração.
- `zonedDateTimeUntil(start, end): Result<Duration, TemporalError>` retorna uma diferença com sinal.
- `zonedDateTimeFormat(value, pattern): Result<string, TemporalError>` aceita `yyyy`, `MM`, `dd`, `HH`, `mm` e `ss`.

<!-- @include: ../../../.fragments/modules/temporal/zoned-date-time/api.md -->
