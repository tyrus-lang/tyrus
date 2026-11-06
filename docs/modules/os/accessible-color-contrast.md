---
title: accessibleColorContrast
description: Check whether a foreground/background color pair meets WCAG contrast guidelines through the native tyrus:os module.
---

# accessibleColorContrast

`accessibleColorContrast` checks whether a foreground/background color pair meets WCAG contrast guidelines.

<SignatureBlock label="Signature">accessibleColorContrast(fg: Color, bg: Color): boolean</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-accessible-color-contrast.md -->
Returns whether `fg`/`bg` clear the WCAG 2.x contrast ratio of `4.5:1`, the AA threshold for normal-size text. The ratio is computed from each color's relative luminance (`(L1 + 0.05) / (L2 + 0.05)`, `L1` the lighter of the two), so which argument is foreground versus background does not change the outcome.

### Color Fields

| Field | Type | Meaning |
|-------|------|---------|
| `r` | `u8` | The red channel, `0` to `255`. |
| `g` | `u8` | The green channel, `0` to `255`. |
| `b` | `u8` | The blue channel, `0` to `255`. |

::: tip Only the AA normal-text threshold
This checks a single fixed threshold (`4.5:1`). WCAG defines other thresholds for large text (`3:1`) and the stricter AAA level (`7:1`/`4.5:1`) that this function does not expose directly, compute the ratio yourself from the same relative-luminance formula if you need one of those instead.
:::
