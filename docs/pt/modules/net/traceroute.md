---
title: traceroute
description: Rastreia saltos IPv4 com sondagens UDP e respostas ICMP.
---

# traceroute

`traceroute` informa saltos IPv4 que respondem dentro de `Ok`; Linux e macOS usam sondagens UDP com TTL crescente e recebem respostas ICMP, enquanto Windows usa sondagens ICMP Echo com TTL crescente. Falhas recuperáveis da sondagem retornam como `Err(NetError)`.

O resultado contém no máximo 30 saltos. Um salto que não responde até o timeout é representado por `"*"`. A função está disponível quando `supportsIcmp()` retorna true. O macOS exige permissão para abrir um socket ICMP raw.

<SignatureBlock label="Assinatura">traceroute(address: string, timeoutMs?: u64): Promise&lt;Result&lt;string[], NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/traceroute/traceroute.md -->

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/net/traceroute/error-handling.md -->
:::

Falhas usam `ERR_NET_INVALID_ADDRESS`, `ERR_NET_PERMISSION`, `ERR_NET_TIMEOUT` ou `ERR_NET_UNSUPPORTED`.
