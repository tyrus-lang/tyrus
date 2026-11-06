---
title: ping
description: Measure one ICMP Echo round trip.
---

# ping

`ping` sends one ICMP Echo request and returns its round-trip time in milliseconds inside `Ok`; recoverable probe failures are returned as `Err(NetError)`.

The function is available when `supportsIcmp()` is true. It is implemented on Linux, macOS, and Windows and returns `Err(NetError)` when the platform or sandbox does not provide the required capability. macOS requires permission to open a raw ICMP socket.

<SignatureBlock label="Signature">ping(address: string, timeoutMs?: u64): Promise&lt;Result&lt;u64, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../.fragments/modules/net/ping/ping.md -->

## Error Handling

::: danger error
<!-- @include: ../../.fragments/modules/net/ping/error-handling.md -->
:::

Failures use `ERR_NET_INVALID_ADDRESS`, `ERR_NET_PERMISSION`, `ERR_NET_TIMEOUT`, or `ERR_NET_UNSUPPORTED`.
