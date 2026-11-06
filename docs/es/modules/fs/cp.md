---
title: cp
description: Copia un archivo o directorio.
---

# cp

Copia un archivo o directorio de `src` a `dest`. Para los directorios, usa `{ recursive: true }`.

<SignatureBlock label="Firma">cp(src: string, dest: string, options?: CpOptions): Result&lt;void, FsError&gt;</SignatureBlock>

La operación devuelve `Err(FsError)` si falla; inspecciónela con `match` o propáguela con `?`.

## Opciones

`CpOptions` es una interfaz con dos campos opcionales:

- `recursive?: boolean`: cuando es `true`, copia directorios de forma recursiva. Es obligatorio para los directorios de origen. El valor predeterminado es `false`.
- `force?: boolean`: cuando es `true`, sobrescribe los archivos existentes en el destino. El valor predeterminado es `false`.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/cp/basic-usage.md -->
