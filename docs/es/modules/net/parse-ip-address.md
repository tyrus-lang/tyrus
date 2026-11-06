---
title: parseIpAddress
description: Analiza una dirección IPv4 o IPv6 sin realizar I/O.
---

# parseIpAddress

`parseIpAddress` analiza una dirección textual IPv4 o IPv6 y devuelve `IpAddress?`.

La función valida toda la entrada sin acceder a la red. El texto inválido produce un opcional ausente.

<SignatureBlock label="Firma">parseIpAddress(value: string): IpAddress?</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/parse-ip-address/parseipaddress.md -->

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/net/parse-ip-address/error-handling.md -->
:::

La entrada inválida se representa mediante el resultado opcional ausente; esta función no lanza `NetError`.
