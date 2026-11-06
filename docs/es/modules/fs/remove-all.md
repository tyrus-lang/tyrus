---
title: removeAll
description: Elimina recursivamente un directorio y todo su contenido.
---

# removeAll

Elimina recursivamente un directorio y todo su contenido. Equivale a `rm(path, { recursive: true })`, pero no acepta opciones.

<SignatureBlock label="Firma">removeAll(path: string): Result&lt;void, FsError&gt;</SignatureBlock>

La operación devuelve `Err(FsError)` si falla; inspecciónela con `match` o propáguela con `?`.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/remove-all/basic-usage.md -->
