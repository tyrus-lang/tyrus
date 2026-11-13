---
title: systemFonts
description: Lista as fontes instaladas no sistema através do módulo nativo tyrus:os.
---

# systemFonts

`systemFonts` lista as fontes instaladas no sistema.

<SignatureBlock label="Assinatura">systemFonts(): FontInfo[]</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-system-fonts.md -->
Retorna um `FontInfo` por face de fonte, uma família com variantes regular/bold/italic/bold-italic contribui com uma entrada por variante, não uma por família.

### Campos de FontInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `name` | `string` | O nome de família primário da face. |
| `style` | `string` | `"normal"`, `"italic"`, ou `"oblique"`. |
| `path` | `string` | O caminho do arquivo de fonte, ou uma string vazia para uma fonte embutida em memória em vez de apoiada por um arquivo. |

::: tip Um array vazio é um resultado válido
Uma máquina headless pode plausivelmente não ter nenhuma fonte instalada, isso não é uma condição de erro.
:::
