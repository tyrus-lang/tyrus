---
title: exists
description: Verifica si una ruta existe mediante el módulo nativo tyrus:fs.
---

# exists

`exists` verifica si una ruta existe en el sistema de archivos y devuelve un `boolean`. `exists` devuelve `false` ante cualquier error (archivo inexistente, permiso denegado, etc.) en lugar de devolver un `Result`.

<SignatureBlock label="Firma">exists(path: string): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/fs/exists/basic-usage.md -->
## Observaciones

`exists` es una comprobación práctica para quien solo necesita saber si una ruta es accesible. Las demás operaciones recuperables devuelven `Result` y deben tratarse con `match` o `?`:

<!-- @include: ../../../.fragments/modules/fs/exists/with-explicit-types.md -->
