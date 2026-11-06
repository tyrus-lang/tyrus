---
title: supportsIcmp
description: Detecta se sondagens de rede ICMP estão disponíveis.
---

# supportsIcmp

`supportsIcmp` informa se o ambiente atual permite os sockets necessários para `ping` e `traceroute`.

No Linux, o resultado verifica as duas capacidades de socket. No macOS, verifica se um socket ICMP raw pode ser aberto. No Windows, verifica se o identificador ICMP nativo do IP Helper pode ser aberto. Em plataformas não suportadas ou sandboxes restritas, retorna `false`.

<SignatureBlock label="Assinatura">supportsIcmp(): boolean</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/supports-icmp/supportsicmp.md -->

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/net/supports-icmp/error-handling.md -->
:::

A verificação de capacidade não lança erros. `ping` e `traceroute` cumprem com `Err(NetError)` usando `ERR_NET_UNSUPPORTED` quando ela retorna `false`.
