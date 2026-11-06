---
title: parseIpAddress
description: Faz parsing de um endereço IPv4 ou IPv6 sem realizar I/O.
---

# parseIpAddress

`parseIpAddress` faz parsing de um endereço textual IPv4 ou IPv6 e retorna `IpAddress?`.

A função valida toda a entrada sem acessar a rede. Texto inválido produz um opcional ausente.

<SignatureBlock label="Assinatura">parseIpAddress(value: string): IpAddress?</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/parse-ip-address/parseipaddress.md -->

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/net/parse-ip-address/error-handling.md -->
:::

Uma entrada inválida é representada pelo resultado opcional ausente; esta função não lança `NetError`.
