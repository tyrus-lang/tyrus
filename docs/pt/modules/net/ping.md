---
title: ping
description: Mede uma ida e volta ICMP Echo.
---

# ping

`ping` envia uma requisição ICMP Echo e retorna o tempo de ida e volta em milissegundos dentro de `Ok`; falhas recuperáveis da sondagem retornam como `Err(NetError)`.

A função está disponível quando `supportsIcmp()` retorna true. Ela é implementada no Linux, macOS e Windows e retorna `Err(NetError)` quando a plataforma ou sandbox não oferece a capacidade necessária. O macOS exige permissão para abrir um socket ICMP raw.

<SignatureBlock label="Assinatura">ping(address: string, timeoutMs?: u64): Promise&lt;Result&lt;u64, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/ping/ping.md -->

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/net/ping/error-handling.md -->
:::

Falhas usam `ERR_NET_INVALID_ADDRESS`, `ERR_NET_PERMISSION`, `ERR_NET_TIMEOUT` ou `ERR_NET_UNSUPPORTED`.
