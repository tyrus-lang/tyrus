---
title: TimeZone
description: El enum IANA versionado en el repositorio y su resolvedor.
---

# TimeZone

`TimeZone` es un enum cerrado generado desde el catálogo IANA versionado en el repositorio. `timeZone` acepta un identificador IANA y valida que la base de datos del runtime lo proporcione.

`timeZone(name: string): Result<TimeZone, TemporalError>` devuelve `Err` para una zona IANA desconocida o no disponible.

<!-- @include: ../../../.fragments/modules/temporal/time-zone/timezone.md -->
