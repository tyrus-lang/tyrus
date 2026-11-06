---
title: resolve
description: Resolve um nome de host pelo resolvedor do sistema.
---

# resolve

`resolve` resolve um nome de host ou endereço em endereços textuais.

A resolução é assíncrona e aceita um timeout opcional em milissegundos. Falhas de rede cumprem a promise com `Err(NetError)`.

<SignatureBlock label="Assinatura">resolve(host: string, timeoutMs?: u64): Promise&lt;Result&lt;string[], NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/resolve/resolve.md -->

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/net/resolve/error-handling.md -->
:::

Falhas comuns incluem `ERR_NET_RESOLVE` e `ERR_NET_TIMEOUT`.
