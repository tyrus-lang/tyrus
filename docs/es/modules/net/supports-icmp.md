---
title: supportsIcmp
description: Detecta si las sondas de red ICMP están disponibles.
---

# supportsIcmp

`supportsIcmp` informa si el entorno actual permite los sockets necesarios para `ping` y `traceroute`.

En Linux, el resultado verifica ambas capacidades de socket. En macOS, verifica si se puede abrir un socket ICMP raw. En Windows, verifica si se puede abrir el identificador ICMP nativo de IP Helper. En plataformas no compatibles o sandboxes restringidos devuelve `false`.

<SignatureBlock label="Firma">supportsIcmp(): boolean</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/supports-icmp/supportsicmp.md -->

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/net/supports-icmp/error-handling.md -->
:::

La comprobación de capacidad no lanza errores. `ping` y `traceroute` cumplen con `Err(NetError)` usando `ERR_NET_UNSUPPORTED` cuando devuelve `false`.
