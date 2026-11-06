---
title: parseSocketAddress
description: Analiza una dirección de socket IPv4 o IPv6 sin realizar I/O.
---

# parseSocketAddress

`parseSocketAddress` analiza una dirección y puerto, incluida la notación IPv6 entre corchetes.

La función valida toda la entrada sin abrir un socket. El texto inválido produce un opcional ausente.

<SignatureBlock label="Firma">parseSocketAddress(value: string): SocketAddress?</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/parse-socket-address/parsesocketaddress.md -->

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/net/parse-socket-address/error-handling.md -->
:::

Los puertos inválidos y las direcciones malformadas producen el resultado opcional ausente; esta función no lanza `NetError`.
