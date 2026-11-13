---
title: tipos de tyrus:net
description: Tipos de handle, paquete, dirección y error exportados por tyrus:net.
---

# Tipos de tyrus:net

`tyrus:net` exporta valores estructurados de dirección y paquete, errores recuperables estructurados y handles explícitos de TCP y UDP.

::: tip Los tipos deben importarse por nombre
`IpAddress`, `SocketAddress`, `UdpPacket`, `UdpSocket`, `TcpStream`, `TcpListener` y `NetError` deben importarse explícitamente siempre que se usen sus campos o métodos.
:::

## IpAddress

`IpAddress` es devuelto por [`parseIpAddress()`](./parse-ip-address.md). Representa una dirección IPv4 o IPv6 validada.

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `address` | `string` | Dirección textual canónica, como `"192.0.2.10"` o `"2001:db8::10"`. |
| `family` | `string` | Familia de dirección: `"IPv4"` o `"IPv6"`. |

<!-- @include: ../../../.fragments/modules/net/types/ipaddress.md -->

## SocketAddress

`SocketAddress` es devuelto por [`parseSocketAddress()`](./parse-socket-address.md). Combina una dirección IP validada con su puerto.

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `ip` | `IpAddress` | La dirección IPv4 o IPv6 analizada. |
| `port` | `u16` | Número de puerto de `0` a `65535`. |

<!-- @include: ../../../.fragments/modules/net/types/socketaddress.md -->

## UdpPacket

`UdpPacket` es devuelto por [`UdpSocket.receiveFrom()`](#udpsocket-methods). Contiene un datagrama recibido y su dirección de origen.

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `data` | `u8[]` | Payload del datagrama, truncado al `maxBytes` solicitado por `receiveFrom`. |
| `address` | `string` | Dirección de socket del remitente, como `"127.0.0.1:50000"`. |

<!-- @include: ../../../.fragments/modules/net/types/udppacket.md -->

## NetError

`NetError` es el tipo de error recuperable de las operaciones asíncronas de red.

`NetError` es un `Error`: un `catch (error)` sin anotación lo recibe, y `error is NetError` estrecha el binding a él, con todos los campos de `NetError`.

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `code` | `string` | Código estable legible por máquina, como `"ERR_NET_TIMEOUT"`. |
| `message` | `string` | Explicación legible del fallo. |

<!-- @include: ../../../.fragments/modules/net/types/neterror.md -->

## Métodos de UdpSocket

`UdpSocket` es devuelto por [`udpBind()`](./udp-bind.md). Ciérralo después de su operación final; las llamadas posteriores devuelven `Err(NetError)` con `ERR_NET_CLOSED`.

| Método | Firma | Descripción |
|--------|-------|-------------|
| `sendTo` | `(data: u8[], address: string, timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Envía un datagrama y devuelve la cantidad de bytes enviados. |
| `receiveFrom` | `(maxBytes: u32, timeoutMs?: u64) => Promise<Result<UdpPacket, NetError>>` | Recibe un datagrama y su dirección de origen. |
| `setBroadcast` | `(enabled: boolean) => Promise<void>` | Habilita o deshabilita la transmisión broadcast. |
| `setTtl` | `(ttl: u32) => Promise<void>` | Establece el TTL IP; los valores válidos son de `1` a `255`. |
| `localAddress` | `() => string` | Devuelve la dirección local vinculada. |
| `close` | `() => void` | Cierra el socket e interrumpe recepciones pendientes. |

<!-- @include: ../../../.fragments/modules/net/types/udpsocket-methods.md -->

## Métodos de TcpStream

`TcpStream` es devuelto por [`tcpConnect()`](./tcp-connect.md) y [`TcpListener.accept()`](#tcplistener-methods). Ciérralo después de su operación final.

| Método | Firma | Descripción |
|--------|-------|-------------|
| `read` | `(maxBytes: u32, timeoutMs?: u64) => Promise<Result<u8[], NetError>>` | Lee hasta `maxBytes`; EOF es un array vacío. |
| `write` | `(data: u8[], timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Escribe bytes y devuelve la cantidad escrita. |
| `shutdown` | `() => Promise<Result<void, NetError>>` | Cierra el lado de escritura manteniendo el lado de lectura. |
| `localAddress` | `() => string` | Devuelve la dirección local del socket. |
| `peerAddress` | `() => string` | Devuelve la dirección remota del socket. |
| `stream` | `() => Promise<Result<DuplexStream<u8[], u8[]>, NetError>>` | Entrega la conexión a un duplex de bytes acotado; cerrar su lado de escritura cierra solo la dirección de escritura. Después, `read`, `write` y `shutdown` devuelven `Err(NetError)` con `ERR_NET_STREAMED`. |
| `close` | `() => void` | Cierra el stream e interrumpe I/O pendiente. |

<!-- @include: ../../../.fragments/modules/net/types/tcpstream-methods.md -->

## Métodos de TcpListener

`TcpListener` es devuelto por [`tcpListen()`](./tcp-listen.md). Ciérralo después de dejar de aceptar conexiones.

| Método | Firma | Descripción |
|--------|-------|-------------|
| `accept` | `(timeoutMs?: u64) => Promise<Result<TcpStream, NetError>>` | Acepta la siguiente conexión. |
| `localAddress` | `() => string` | Devuelve la dirección local en escucha. |
| `close` | `() => void` | Cierra el listener e interrumpe accepts pendientes. |

<!-- @include: ../../../.fragments/modules/net/types/tcplistener-methods.md -->

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/net/types/error-handling.md -->
:::
