---
title: parseSocketAddress
description: Parse an IPv4 or IPv6 socket address without performing I/O.
---

# parseSocketAddress

`parseSocketAddress` parses an address and port, including bracketed IPv6 notation.

The function validates the complete input without opening a socket. Invalid text produces an absent optional value.

<SignatureBlock label="Signature">parseSocketAddress(value: string): SocketAddress?</SignatureBlock>

<!-- @include: ../../.fragments/modules/net/parse-socket-address/parsesocketaddress.md -->

## Error Handling

::: danger error
<!-- @include: ../../.fragments/modules/net/parse-socket-address/error-handling.md -->
:::

Invalid ports and malformed addresses produce the absent optional result; this function does not throw `NetError`.
