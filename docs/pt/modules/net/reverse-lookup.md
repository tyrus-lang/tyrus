---
title: reverseLookup
description: Resolve um endereço textual de volta para um nome de host.
---

# reverseLookup

`reverseLookup` realiza uma resolução reversa de nome para um endereço.

A resolução é assíncrona e aceita um timeout opcional em milissegundos. Falhas de rede cumprem a promise com `Err(NetError)`.

<SignatureBlock label="Assinatura">reverseLookup(address: string, timeoutMs?: u64): Promise&lt;Result&lt;string, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/reverse-lookup/reverselookup.md -->

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/net/reverse-lookup/error-handling.md -->
:::

Falhas comuns incluem `ERR_NET_INVALID_ADDRESS`, `ERR_NET_RESOLVE` e `ERR_NET_TIMEOUT`.
