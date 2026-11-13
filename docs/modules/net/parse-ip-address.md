---
title: parseIpAddress
description: Parse an IPv4 or IPv6 address without performing I/O.
---

# parseIpAddress

`parseIpAddress` parses a textual IPv4 or IPv6 address and returns an `IpAddress?`.

The function validates the complete input without contacting the network. Invalid text produces an absent optional value.

<SignatureBlock label="Signature">parseIpAddress(value: string): IpAddress?</SignatureBlock>

<!-- @include: ../../.fragments/modules/net/parse-ip-address/parseipaddress.md -->

## Error Handling

::: danger error
<!-- @include: ../../.fragments/modules/net/parse-ip-address/error-handling.md -->
:::

Invalid input is represented by the absent optional result; this function does not throw `NetError`.
