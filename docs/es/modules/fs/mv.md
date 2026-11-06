---
title: mv
description: Mueve (renombra) un archivo o directorio.
---

# mv

Mueve (renombra) un archivo o directorio de `src` a `dest` en el mismo sistema de archivos. Los movimientos entre dispositivos devuelven `ERR_FS_MV_FAILED`; el archivo de origen permanece en su lugar.

<SignatureBlock label="Firma">mv(src: string, dest: string): Result&lt;void, FsError&gt;</SignatureBlock>

La operación devuelve `Err(FsError)` si falla; inspecciónela con `match` o propáguela con `?`.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/mv/basic-usage.md -->
