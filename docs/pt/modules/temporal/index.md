---
title: Temporal
description: Valores nativos de data e hora expostos por tyrus:temporal.
---

# Temporal

`tyrus:temporal` fornece valores nativos para instantes absolutos, datas de calendário, horários, durações, datas e horas com fuso e fusos horários IANA versionados no repositório. Todas as APIs nativas falíveis do módulo retornam falhas recuperáveis como `Result<T, TemporalError>`; operações sem falha, como igualdade de valores, retornam diretamente o valor.

## APIs disponíveis

| API | Finalidade |
|-----|------------|
| [`Instant`](./instant.md) | Pontos absolutos na linha do tempo. |
| [`PlainDate`](./plain-date.md) | Datas de calendário sem hora ou fuso horário. |
| [`PlainTime`](./plain-time.md) | Horários sem data ou fuso horário. |
| [`PlainDateTime`](./plain-date-time.md) | Datas de calendário combinadas com horários. |
| [`ZonedDateTime`](./zoned-date-time.md) | Instantes associados a um fuso horário IANA. |
| [`Duration`](./duration.md) | Unidades de calendário e relógio com sinal. |
| [`TimeZone`](./time-zone.md) | O enum IANA versionado no repositório e seu resolvedor. |
| [`PlainYearMonth`](./plain-year-month.md) | Valores de ano e mês do calendário. |
| [`PlainMonthDay`](./plain-month-day.md) | Valores recorrentes de mês e dia. |

## Importação

<SignatureBlock label="Importações">import { Instant, PlainDate, TimeZone, instantNow, plainDateFrom, timeZone } from "tyrus:temporal";</SignatureBlock>
