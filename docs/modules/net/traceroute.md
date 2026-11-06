---
title: traceroute
description: Trace IPv4 hops with UDP probes and ICMP responses.
---

# traceroute

`traceroute` reports responding IPv4 hops inside `Ok`; Linux and macOS use UDP probes with increasing TTL values and receive ICMP responses, while Windows uses ICMP Echo probes with increasing TTL values. Recoverable probe failures are returned as `Err(NetError)`.

The result contains at most 30 hops. A hop that does not answer before the timeout is represented by `"*"`. The function is available when `supportsIcmp()` is true. macOS requires permission to open a raw ICMP socket.

<SignatureBlock label="Signature">traceroute(address: string, timeoutMs?: u64): Promise&lt;Result&lt;string[], NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../.fragments/modules/net/traceroute/traceroute.md -->

## Error Handling

::: danger error
<!-- @include: ../../.fragments/modules/net/traceroute/error-handling.md -->
:::

Failures use `ERR_NET_INVALID_ADDRESS`, `ERR_NET_PERMISSION`, `ERR_NET_TIMEOUT`, or `ERR_NET_UNSUPPORTED`.
