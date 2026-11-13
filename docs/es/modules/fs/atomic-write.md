---
title: atomicWrite
description: Escribe un archivo de forma atómica.
---

# atomicWrite

Escribe datos en un archivo de forma atómica: primero en un archivo temporal único dentro del directorio de destino y luego reemplaza el destino. Así se evitan escrituras parciales en caso de fallo y funciona aunque el archivo ya exista en Windows, macOS y Linux.

<SignatureBlock label="Firma">atomicWrite(path: string, data: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/atomic-write/example.md -->
