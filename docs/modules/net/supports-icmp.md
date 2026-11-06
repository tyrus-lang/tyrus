---
title: supportsIcmp
description: Detect whether ICMP network probes are available.
---

# supportsIcmp

`supportsIcmp` reports whether the current environment permits the sockets required by `ping` and `traceroute`.

On Linux, the result checks both socket capabilities. On macOS, it checks whether a raw ICMP socket can be opened. On Windows, it checks whether the native IP Helper ICMP handle can be opened. On unsupported platforms or restricted sandboxes it returns `false`.

<SignatureBlock label="Signature">supportsIcmp(): boolean</SignatureBlock>

<!-- @include: ../../.fragments/modules/net/supports-icmp/supportsicmp.md -->

## Error Handling

::: danger error
<!-- @include: ../../.fragments/modules/net/supports-icmp/error-handling.md -->
:::

The capability check itself does not throw. `ping` and `traceroute` fulfill with `Err(NetError)` using `ERR_NET_UNSUPPORTED` when it returns `false`.
