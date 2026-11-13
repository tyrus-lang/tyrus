---
title: accessibleColorContrast
description: Verifica si un par de colores de primer plano/fondo cumple las directrices de contraste WCAG mediante el módulo nativo tyrus:os.
---

# accessibleColorContrast

`accessibleColorContrast` verifica si un par de colores de primer plano/fondo cumple las directrices de contraste WCAG.

<SignatureBlock label="Firma">accessibleColorContrast(fg: Color, bg: Color): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-accessible-color-contrast.md -->
Devuelve si `fg`/`bg` alcanzan la proporción de contraste WCAG 2.x de `4.5:1`, el umbral AA para texto de tamaño normal. La proporción se calcula a partir de la luminancia relativa de cada color (`(L1 + 0.05) / (L2 + 0.05)`, siendo `L1` el más claro de los dos), así que cuál argumento es primer plano frente a fondo no cambia el resultado.

### Campos de Color

| Campo | Tipo | Significado |
|-------|------|-------------|
| `r` | `u8` | El canal rojo, de `0` a `255`. |
| `g` | `u8` | El canal verde, de `0` a `255`. |
| `b` | `u8` | El canal azul, de `0` a `255`. |

::: tip Solo el umbral AA de texto normal
Esto verifica un único umbral fijo (`4.5:1`). WCAG define otros umbrales para texto grande (`3:1`) y el nivel AAA más estricto (`7:1`/`4.5:1`) que esta función no expone directamente, calcula la proporción tú mismo con la misma fórmula de luminancia relativa si necesitas uno de esos.
:::
