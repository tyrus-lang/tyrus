---
title: accessibleColorContrast
description: Verifica se um par de cores de primeiro plano/fundo atende às diretrizes de contraste WCAG através do módulo nativo tyrus:os.
---

# accessibleColorContrast

`accessibleColorContrast` verifica se um par de cores de primeiro plano/fundo atende às diretrizes de contraste WCAG.

<SignatureBlock label="Assinatura">accessibleColorContrast(fg: Color, bg: Color): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-accessible-color-contrast.md -->
Retorna se `fg`/`bg` atingem a proporção de contraste WCAG 2.x de `4.5:1`, o limiar AA para texto de tamanho normal. A proporção é calculada a partir da luminância relativa de cada cor (`(L1 + 0.05) / (L2 + 0.05)`, `L1` sendo a mais clara das duas), então qual argumento é primeiro plano versus fundo não muda o resultado.

### Campos de Color

| Campo | Tipo | Significado |
|-------|------|-------------|
| `r` | `u8` | O canal vermelho, de `0` a `255`. |
| `g` | `u8` | O canal verde, de `0` a `255`. |
| `b` | `u8` | O canal azul, de `0` a `255`. |

::: tip Só o limiar AA de texto normal
Isto verifica um único limiar fixo (`4.5:1`). O WCAG define outros limiares para texto grande (`3:1`) e o nível AAA mais estrito (`7:1`/`4.5:1`) que esta função não expõe diretamente, calcule a proporção você mesmo a partir da mesma fórmula de luminância relativa se precisar de um desses.
:::
