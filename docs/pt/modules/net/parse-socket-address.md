---
title: parseSocketAddress
description: Faz parsing de um endereço de socket IPv4 ou IPv6 sem realizar I/O.
---

# parseSocketAddress

`parseSocketAddress` faz parsing de um endereço e porta, incluindo a notação IPv6 entre colchetes.

A função valida toda a entrada sem abrir um socket. Texto inválido produz um opcional ausente.

<SignatureBlock label="Assinatura">parseSocketAddress(value: string): SocketAddress?</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/parse-socket-address/parsesocketaddress.md -->

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/net/parse-socket-address/error-handling.md -->
:::

Portas inválidas e endereços malformados produzem o resultado opcional ausente; esta função não lança `NetError`.
