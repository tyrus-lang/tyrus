---
title: rm
description: Elimina un archivo o directorio.
---

# rm

Elimina un archivo o directorio. Para los directorios, usa `{ recursive: true }` para eliminar todo el contenido.

<SignatureBlock label="Firma">rm(path: string, options?: RmOptions): Result&lt;void, FsError&gt;</SignatureBlock>

La operación devuelve `Err(FsError)` si falla; inspecciónela con `match` o propáguela con `?`.

## Opciones

`RmOptions` es una interfaz con dos campos opcionales:

- `recursive?: boolean`: cuando es `true`, elimina directorios y todo su contenido de forma recursiva. El valor predeterminado es `false`.
- `force?: boolean`: cuando es `true`, ignora las rutas inexistentes sin generar un error. El valor predeterminado es `false`.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/rm/basic-usage.md -->
