---
title: locale
description: Leia o código de locale atual através do módulo nativo tyrus:os.
---

# locale

`locale` devolve o código de locale atual (ex.: `"en-US"`, `"pt-BR"`).

<SignatureBlock label="Assinatura">locale(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-locale.md -->
::: tip Nunca falha o programa
Cai para `"und"`, o subtag padrão de "locale indeterminado", se o locale reportado pelo SO não puder ser lido, em vez de abortar o programa.
:::

Veja também [`language`](./language.md), que devolve só o subtag de idioma inicial.
