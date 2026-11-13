---
title: mime
description: Obtiene el tipo MIME por la extensión del archivo.
---

# mime

Devuelve el tipo MIME de un archivo según su extensión. No devuelve un valor cuando la extensión es desconocida o la ruta no tiene extensión. A diferencia de las operaciones que devuelven `Result`, `mime` no tiene fallos recuperables.

<SignatureBlock label="Firma">mime(path: string): string?</SignatureBlock>

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/mime/example.md -->
