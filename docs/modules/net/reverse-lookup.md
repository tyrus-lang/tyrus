---
title: reverseLookup
description: Resolve a textual address back to a host name.
---

# reverseLookup

`reverseLookup` performs a reverse name lookup for an address.

The lookup runs asynchronously and accepts an optional timeout in milliseconds. Network failures fulfill the promise with `Err(NetError)`.

<SignatureBlock label="Signature">reverseLookup(address: string, timeoutMs?: u64): Promise&lt;Result&lt;string, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../.fragments/modules/net/reverse-lookup/reverselookup.md -->

## Error Handling

::: danger error
<!-- @include: ../../.fragments/modules/net/reverse-lookup/error-handling.md -->
:::

Typical failures include `ERR_NET_INVALID_ADDRESS`, `ERR_NET_RESOLVE`, and `ERR_NET_TIMEOUT`.
