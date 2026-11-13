---
title: TimeZone
description: O enum IANA versionado no repositório e seu resolvedor.
---

# TimeZone

`TimeZone` é um enum fechado gerado a partir do catálogo IANA versionado no repositório. `timeZone` aceita um identificador IANA e valida se o banco de dados do runtime o fornece.

`timeZone(name: string): Result<TimeZone, TemporalError>` retorna `Err` para um fuso IANA desconhecido ou indisponível.

<!-- @include: ../../../.fragments/modules/temporal/time-zone/timezone.md -->
