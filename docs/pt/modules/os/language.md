---
title: language
description: Leia o subtag de idioma do locale atual através do módulo nativo tyrus:os.
---

# language

`language` devolve o subtag de idioma do locale atual (ex.: `"en"`, `"pt"`), a parte inicial do código completo de [`locale`](./locale.md).

<SignatureBlock label="Assinatura">language(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-language.md -->
::: tip Nunca falha o programa
Cai para `"und"`, o subtag padrão de "locale indeterminado", se o locale reportado pelo SO não puder ser lido, em vez de abortar o programa.
:::
