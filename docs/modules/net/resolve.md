---
title: resolve
description: Resolve a host name through the system resolver.
---

# resolve

`resolve` resolves a host name or address into textual addresses.

Resolution runs asynchronously and accepts an optional timeout in milliseconds. Network failures fulfill the promise with `Err(NetError)`.

<SignatureBlock label="Signature">resolve(host: string, timeoutMs?: u64): Promise&lt;Result&lt;string[], NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../.fragments/modules/net/resolve/resolve.md -->

## Error Handling

::: danger error
<!-- @include: ../../.fragments/modules/net/resolve/error-handling.md -->
:::

Typical failures include `ERR_NET_RESOLVE` and `ERR_NET_TIMEOUT`.
