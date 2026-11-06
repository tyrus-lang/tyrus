---
title: PlainTime
description: Horários sem data ou fuso horário.
---

# PlainTime

`PlainTime` representa um horário sem data ou fuso horário. A adição atravessa a meia-noite e preserva a precisão de nanossegundos.

## API

- `plainTimeFrom(value: string): Result<PlainTime, TemporalError>` analisa um texto de relógio ISO.
- `plainTimeFromFields(fields: PlainTime): Result<PlainTime, TemporalError>` valida os campos do horário.
- `plainTimeToString(value: PlainTime): Result<string, TemporalError>` formata o horário.
- `plainTimeEquals(left, right): boolean` compara horários por valor.
- `plainTimeAddSeconds` e `plainTimeAddNanoseconds` retornam `Result<PlainTime, TemporalError>` após adicionar unidades com sinal e fazer o horário circular.
- `plainTimeAddDuration` e `plainTimeSubtractDuration` retornam `Result<PlainTime, TemporalError>` após aplicar uma `Duration` de relógio.
- `plainTimeUntil` e `plainTimeSince` retornam diferenças em `Result<Duration, TemporalError>`.
- `plainTimeUntilWithOptions` e `plainTimeSinceWithOptions` retornam `Result<Duration, TemporalError>` e aceitam `largestUnit`, `smallestUnit`, `roundingIncrement` e `roundingMode`.
- `plainTimeSecondsUntil` e `plainTimeNanosecondsUntil` retornam diferenças escalares em `Result<i64, TemporalError>`.

<!-- @include: ../../../.fragments/modules/temporal/plain-time/api.md -->
