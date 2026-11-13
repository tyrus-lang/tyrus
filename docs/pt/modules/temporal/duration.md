---
title: Duration
description: Unidades de calendário e relógio com sinal.
---

# Duration

`Duration` armazena unidades de calendário e relógio com sinal. Use `durationTotal` apenas com unidades de tamanho fixo; anos e meses exigem uma referência de calendário.

## API

- `durationFromFields(fields: Duration): Duration` valida e retorna os campos da duração.
- `durationTotal(value, unit): Result<i64, TemporalError>` converte unidades de tamanho fixo, como segundos, minutos, horas e dias.
- `plainDateAddDuration`, `plainDateSubtractDuration`, `plainTimeAddDuration`, `plainTimeSubtractDuration` e `zonedDateTimeAddDuration` aplicam durações a valores temporais.
- `plainDateUntil`, `plainDateSince`, `plainTimeUntil`, `plainTimeSince` e `zonedDateTimeUntil` produzem durações.
- `durationRound(value, unit, increment, mode, relativeTo): Result<Duration, TemporalError>` arredonda uma duração; unidades de calendário exigem a data explícita `relativeTo`.

<!-- @include: ../../../.fragments/modules/temporal/duration/api.md -->
