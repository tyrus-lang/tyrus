---
title: appDataDir
description: Lee un directorio de datos específico de aplicación mediante el módulo nativo tyrus:os.
---

# appDataDir

`appDataDir` devuelve un subdirectorio específico de aplicación dentro de [`dataDir`](./data-dir.md) para `app`.

<SignatureBlock label="Firma">appDataDir(app: string): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-app-data-dir.md -->
::: tip Nunca falla el programa
Recurre a una cadena vacía si no puede determinarse, en lugar de abortar el programa.
:::
