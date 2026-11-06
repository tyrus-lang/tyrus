---
title: tipos de tyrus:net
description: Tipos de handle, pacote, endereço e erro exportados por tyrus:net.
---

# Tipos de tyrus:net

`tyrus:net` exporta valores estruturados de endereço e pacote, erros recuperáveis estruturados e handles explícitos de TCP e UDP.

::: tip Os tipos devem ser importados por nome
`IpAddress`, `SocketAddress`, `UdpPacket`, `UdpSocket`, `TcpStream`, `TcpListener` e `NetError` devem ser importados explicitamente sempre que seus campos ou métodos forem usados.
:::

## IpAddress

`IpAddress` é retornado por [`parseIpAddress()`](./parse-ip-address.md). Ele representa um endereço IPv4 ou IPv6 validado.

| Campo | Tipo | Descrição |
|-------|------|-----------|
| `address` | `string` | Endereço textual canônico, como `"192.0.2.10"` ou `"2001:db8::10"`. |
| `family` | `string` | Família do endereço: `"IPv4"` ou `"IPv6"`. |

<!-- @include: ../../../.fragments/modules/net/types/ipaddress.md -->

## SocketAddress

`SocketAddress` é retornado por [`parseSocketAddress()`](./parse-socket-address.md). Ele combina um endereço IP validado com sua porta.

| Campo | Tipo | Descrição |
|-------|------|-----------|
| `ip` | `IpAddress` | O endereço IPv4 ou IPv6 analisado. |
| `port` | `u16` | Número da porta de `0` até `65535`. |

<!-- @include: ../../../.fragments/modules/net/types/socketaddress.md -->

## UdpPacket

`UdpPacket` é retornado por [`UdpSocket.receiveFrom()`](#udpsocket-methods). Ele contém um datagrama recebido e seu endereço de origem.

| Campo | Tipo | Descrição |
|-------|------|-----------|
| `data` | `u8[]` | Payload do datagrama, truncado para o `maxBytes` solicitado por `receiveFrom`. |
| `address` | `string` | Endereço de socket do remetente, como `"127.0.0.1:50000"`. |

<!-- @include: ../../../.fragments/modules/net/types/udppacket.md -->

## NetError

`NetError` é o tipo de erro recuperável das operações assíncronas de rede.

`NetError` é um `Error`: um `catch (error)` sem anotação o recebe, e `error is NetError` estreita o binding para ele, com todos os campos de `NetError`.

| Campo | Tipo | Descrição |
|-------|------|-----------|
| `code` | `string` | Código estável legível por máquina, como `"ERR_NET_TIMEOUT"`. |
| `message` | `string` | Explicação legível da falha. |

<!-- @include: ../../../.fragments/modules/net/types/neterror.md -->

## Métodos de UdpSocket

`UdpSocket` é retornado por [`udpBind()`](./udp-bind.md). Feche-o após sua operação final; chamadas posteriores retornam `Err(NetError)` com `ERR_NET_CLOSED`.

| Método | Assinatura | Descrição |
|--------|------------|-----------|
| `sendTo` | `(data: u8[], address: string, timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Envia um datagrama e retorna a quantidade de bytes enviados. |
| `receiveFrom` | `(maxBytes: u32, timeoutMs?: u64) => Promise<Result<UdpPacket, NetError>>` | Recebe um datagrama e seu endereço de origem. |
| `setBroadcast` | `(enabled: boolean) => Promise<void>` | Habilita ou desabilita transmissão por broadcast. |
| `setTtl` | `(ttl: u32) => Promise<void>` | Define o TTL IP; valores válidos são de `1` a `255`. |
| `localAddress` | `() => string` | Retorna o endereço local associado. |
| `close` | `() => void` | Fecha o socket e interrompe recebimentos pendentes. |

<!-- @include: ../../../.fragments/modules/net/types/udpsocket-methods.md -->

## Métodos de TcpStream

`TcpStream` é retornado por [`tcpConnect()`](./tcp-connect.md) e [`TcpListener.accept()`](#tcplistener-methods). Feche-o após sua operação final.

| Método | Assinatura | Descrição |
|--------|------------|-----------|
| `read` | `(maxBytes: u32, timeoutMs?: u64) => Promise<Result<u8[], NetError>>` | Lê até `maxBytes`; EOF é um array vazio. |
| `write` | `(data: u8[], timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Escreve bytes e retorna a quantidade escrita. |
| `shutdown` | `() => Promise<Result<void, NetError>>` | Fecha o lado de escrita mantendo o lado de leitura. |
| `localAddress` | `() => string` | Retorna o endereço local do socket. |
| `peerAddress` | `() => string` | Retorna o endereço remoto do socket. |
| `stream` | `() => Promise<Result<DuplexStream<u8[], u8[]>, NetError>>` | Entrega a conexão a um duplex de bytes limitado; fechar o lado de escrita encerra só a direção de escrita. Depois disso, `read`, `write` e `shutdown` retornam `Err(NetError)` com `ERR_NET_STREAMED`. |
| `close` | `() => void` | Fecha o stream e interrompe I/O pendente. |

<!-- @include: ../../../.fragments/modules/net/types/tcpstream-methods.md -->

## Métodos de TcpListener

`TcpListener` é retornado por [`tcpListen()`](./tcp-listen.md). Feche-o depois que parar de aceitar conexões.

| Método | Assinatura | Descrição |
|--------|------------|-----------|
| `accept` | `(timeoutMs?: u64) => Promise<Result<TcpStream, NetError>>` | Aceita a próxima conexão. |
| `localAddress` | `() => string` | Retorna o endereço local em escuta. |
| `close` | `() => void` | Fecha o listener e interrompe accepts pendentes. |

<!-- @include: ../../../.fragments/modules/net/types/tcplistener-methods.md -->

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/net/types/error-handling.md -->
:::
