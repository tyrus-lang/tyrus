---
title: canonicalize
description: Resuelve una ruta a una ruta absoluta.
---

# canonicalize

Resuelve una ruta a una ruta absoluta y normalizada, resolviendo todos los enlaces simbólicos. Devuelve `FsError` si la ruta no existe.

<SignatureBlock label="Firma">canonicalize(path: string): Result&lt;string, FsError&gt;</SignatureBlock>

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/canonicalize/example.md -->
