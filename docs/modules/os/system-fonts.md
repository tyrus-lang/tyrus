---
title: systemFonts
description: List the fonts installed on the system through the native tyrus:os module.
---

# systemFonts

`systemFonts` lists the fonts installed on the system.

<SignatureBlock label="Signature">systemFonts(): FontInfo[]</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-system-fonts.md -->
Returns one `FontInfo` per font face, a family with regular/bold/italic/bold-italic variants contributes one entry per variant, not one per family.

### FontInfo Fields

| Field | Type | Meaning |
|-------|------|---------|
| `name` | `string` | The face's primary family name. |
| `style` | `string` | Either `"normal"`, `"italic"`, or `"oblique"`. |
| `path` | `string` | The font file's path, or an empty string for a font embedded in memory rather than backed by a file. |

::: tip An empty array is a valid outcome
A headless machine can plausibly have zero fonts installed, that is not an error condition.
:::
