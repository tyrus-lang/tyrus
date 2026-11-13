---
title: unpack
description: Extrae de forma segura un archivo ZIP, TAR o TAR.GZ en un directorio.
---

# unpack

`unpack` extrae un archivo ZIP, TAR o TAR.GZ en un directorio de destino y lo crea cuando no existe. El formato se identifica por el contenido, nunca por el nombre del archivo.

<SignatureBlock label="Firma">unpack(archive: string, destination: string, options?: UnpackOptions): Promise&lt;Result&lt;ArchiveSummary, ArchiveError&gt;&gt;</SignatureBlock>

## Ejemplo mínimo

<!-- @include: ../../../.fragments/modules/archive/unpack/minimal.md -->

## Todas las opciones

<!-- @include: ../../../.fragments/modules/archive/unpack/maximal.md -->

| Opción | Tipo | Por defecto | Efecto |
|--------|------|---------|--------|
| `overwrite` | `boolean` | `false` | Reemplaza archivos que ya existen en el destino. Los directorios y enlaces nunca se reemplazan. |
| `stripComponents` | `u32` | `0` | Elimina esa cantidad de componentes iniciales de la ruta de cada entrada; las entradas que quedan vacías se omiten. |
| `maxEntries` | `u64` | `100000` | Rechaza archivos con más entradas. |
| `maxBytes` | `u64` | `4 GiB` | Rechaza la extracción que escribiría más bytes, contados a medida que se escriben. |
| `maxRatio` | `u32` | `200` | Rechaza una entrada ZIP que se expande más de esa cantidad de veces su tamaño comprimido, y un archivo TAR.GZ cuyos bytes escritos superan esa cantidad de veces los bytes comprimidos leídos hasta ese momento, una vez escrito más de 1 MiB. |
| `timeoutMs` | `u64` | `600000` (10 minutos) | Desiste con `ERR_ARCHIVE_TIMEOUT` cuando la operación tarda más, eliminando lo que creó. |

Los límites pueden aumentarse, pero nunca desactivarse: `0` falla con `ERR_ARCHIVE_INVALID_OPTION`.

## Seguridad

- Todo nombre de entrada se valida antes de crear cualquier cosa: se rechazan rutas absolutas, componentes `..`, prefijos de unidad, barras invertidas y nombres que alguna plataforma no puede crear, como `CON` o nombres que terminan en punto.
- Las escrituras permanecen dentro del destino incluso cuando ya contiene un enlace simbólico: la extracción nunca atraviesa un enlace.
- Los enlaces simbólicos, enlaces duros, dispositivos y otras entradas especiales se rechazan en lugar de recrearse.
- Dos entradas que resultan en la misma ruta, incluidos los nombres que solo difieren en mayúsculas o en la normalización Unicode, se rechazan.
- Cada archivo se escribe con un nombre temporal y se renombra al terminar, y ante cualquier falla se eliminan los archivos y directorios creados por esta llamada. El contenido que ya existía en el destino queda intacto.
- Los archivos extraídos nunca reciben bits de permiso especiales; solo se conserva el permiso de ejecución del propietario, para que los programas extraídos sigan siendo ejecutables.
- Antes de extraer un archivo ZIP, los tamaños que declara, limitados a `maxBytes`, se comparan con el espacio libre en el destino, y un archivo que no cabe se rechaza con `ERR_ARCHIVE_NO_SPACE` antes de crear cualquier archivo. Fuera de eso los tamaños nunca se consideran confiables: los límites cuentan los bytes realmente escritos, y un disco que se llena durante la extracción también rechaza con `ERR_ARCHIVE_NO_SPACE`.
- Los archivos comprimidos dentro del archivo nunca se extraen: una entrada `.zip` o `.tar.gz` se escribe como un archivo común.
- `unpack` nunca ejecuta, abre ni interpreta nada de lo que extrae.

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/archive/unpack/error-handling.md -->
:::
