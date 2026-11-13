---
title: systemFonts
description: Lista las fuentes instaladas en el sistema mediante el módulo nativo tyrus:os.
---

# systemFonts

`systemFonts` lista las fuentes instaladas en el sistema.

<SignatureBlock label="Firma">systemFonts(): FontInfo[]</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-system-fonts.md -->
Devuelve un `FontInfo` por cada face de fuente, una familia con variantes regular/bold/italic/bold-italic contribuye con una entrada por variante, no una por familia.

### Campos de FontInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `name` | `string` | El nombre de familia principal de la face. |
| `style` | `string` | `"normal"`, `"italic"`, o `"oblique"`. |
| `path` | `string` | La ruta del archivo de fuente, o una cadena vacía para una fuente incrustada en memoria en lugar de respaldada por un archivo. |

::: tip Un array vacío es un resultado válido
Una máquina headless plausiblemente puede no tener ninguna fuente instalada, eso no es una condición de error.
:::
