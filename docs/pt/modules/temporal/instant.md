---
title: Instant
description: Pontos absolutos na linha do tempo do tyrus.
---

# Instant

`Instant` representa um ponto absoluto na linha do tempo com segundos com sinal e um resto de nanossegundos não negativo.

Quando o Tyrus serializa um `Instant` como JSON, usa a string ISO 8601 em UTC retornada por `instantToString`, inclusive em respostas HTTP. O OpenAPI descreve esse valor como `type: "string"` com `format: "date-time"`, correspondente a valores como `2026-09-05T18:32:52.54345Z`.

## API

- `instantNow(): Instant` lê o relógio nativo atual.
- `instantFrom(value: string): Result<Instant, TemporalError>` analisa um instante ISO.
- `instantFromEpochMilliseconds(milliseconds: i64): Result<Instant, TemporalError>` cria um instante a partir de milissegundos desde o Unix Epoch.
- `instantToEpochMilliseconds(value: Instant): Result<i64, TemporalError>` converte um instante para milissegundos desde o epoch.
- `instantToString(value: Instant): Result<string, TemporalError>` formata um instante como texto ISO UTC.
- `instantAddDuration` e `instantSubtractDuration` retornam `Result<Instant, TemporalError>` e aplicam durações de tamanho fixo; unidades de calendário são rejeitadas.
- `instantToZonedDateTime(value: Instant, timeZone: TimeZone): ZonedDateTime` associa um instante a um fuso IANA versionado no repositório.

<!-- @include: ../../../.fragments/modules/temporal/instant/api.md -->
