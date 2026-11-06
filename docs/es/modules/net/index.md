---
title: tyrus:net
description: APIs nativas de direcciones, resolución de nombres, TCP y UDP asíncronos y sondeos de red condicionados por capacidad.
---

# tyrus:net

`tyrus:net` proporciona parsing validado de direcciones, resolución de nombres del sistema, comunicación TCP y UDP asíncrona y sondeos de red condicionados por capacidad para programas nativos.

## Imports

<SignatureBlock label="Importaciones">import { NetError, TcpListener, TcpStream, UdpPacket, UdpSocket, tcpConnect, tcpListen, udpBind, ping, traceroute, supportsIcmp, parseIpAddress, parseSocketAddress, resolve, reverseLookup } from "tyrus:net";</SignatureBlock>

## Parsing y resolución de direcciones

| Elemento | Firma | Finalidad |
|----------|-------|-----------|
| [`parseIpAddress`](./parse-ip-address.md) | `(value: string) => IpAddress?` | Analiza una dirección IPv4 o IPv6 sin realizar I/O. |
| [`parseSocketAddress`](./parse-socket-address.md) | `(value: string) => SocketAddress?` | Analiza una dirección con puerto, incluida la notación IPv6 entre corchetes. |
| [`resolve`](./resolve.md) | `(host: string, timeoutMs?: u64) => Promise<Result<string[], NetError>>` | Resuelve un host mediante el resolvedor del sistema. |
| [`reverseLookup`](./reverse-lookup.md) | `(address: string, timeoutMs?: u64) => Promise<Result<string, NetError>>` | Realiza una búsqueda inversa de nombre para una dirección. |

El texto de dirección inválido devuelve un opcional ausente en las funciones de parsing. `resolve` y `reverseLookup` cumplen con `Err(NetError)` cuando falla la resolución, y `ping` y `traceroute` hacen lo mismo ante fallos recuperables de las sondas; el `code` estable identifica el fallo y el `message` es legible.

## Contrato y soporte

Los handles de red son tipos importados explícitos. Los métodos toman prestado el handle, por lo que el socket permanece utilizable hasta `close()`; cerrar un handle TCP interrumpe el I/O pendiente y las operaciones posteriores devuelven `Err(NetError)` con `ERR_NET_CLOSED`, y los fallos UDP también devuelven `Err(NetError)`. Las operaciones TCP y UDP pueden ejecutarse de forma concurrente, pero cada operación del socket se serializa mediante el handle. `NetworkInterface` continúa siendo proporcionada por `tyrus:os` para evitar un tipo duplicado de interfaz de red.

| Código | Significado |
|------|---------|
| `ERR_NET_INVALID_ADDRESS` | El texto de la dirección no es válido. |
| `ERR_NET_TIMEOUT` | La operación superó su timeout. |
| `ERR_NET_PERMISSION` | El sistema operativo o sandbox denegó la operación. |
| `ERR_NET_CLOSED` | El socket fue cerrado. |
| `ERR_NET_UNSUPPORTED` | La plataforma actual no tiene implementación para la operación. |

La matriz inicial de soporte es Linux y Windows para parsing de direcciones, resolución, TCP, UDP, ping y traceroute; macOS admite parsing de direcciones, resolución, TCP y UDP, mientras que los sondeos ICMP devuelven `Err(NetError)` con `ERR_NET_UNSUPPORTED`.

## TCP

`tcpConnect(address, timeoutMs?)` devuelve `Promise<Result<TcpStream, NetError>>`. `tcpListen(address, timeoutMs?)` devuelve `Promise<Result<TcpListener, NetError>>`. El timeout opcional se expresa en milisegundos.

| Elemento | Firma | Finalidad |
|----------|-------|-----------|
| `TcpStream.read` | `(maxBytes: u32, timeoutMs?: u64) => Promise<Result<u8[], NetError>>` | Lee hasta la cantidad solicitada de bytes; EOF es un array vacío. |
| `TcpStream.write` | `(data: u8[], timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Escribe bytes y devuelve la cantidad escrita, que puede ser menor que la entrada. |
| `TcpStream.shutdown` | `() => Promise<Result<void, NetError>>` | Apaga el lado de escritura del stream. |
| `TcpStream.localAddress` | `() => string` | Devuelve la dirección local del socket. |
| `TcpStream.peerAddress` | `() => string` | Devuelve la dirección remota del socket. |
| `TcpStream.stream` | `() => Promise<Result<DuplexStream<u8[], u8[]>, NetError>>` | Entrega la conexión a un duplex de bytes acotado. |
| `TcpStream.close` | `() => void` | Cierra el stream y despierta el I/O pendiente. |
| `TcpListener.accept` | `(timeoutMs?: u64) => Promise<Result<TcpStream, NetError>>` | Acepta la siguiente conexión. |
| `TcpListener.localAddress` | `() => string` | Devuelve la dirección del socket en escucha. |
| `TcpListener.close` | `() => void` | Cierra el listener y despierta un accept pendiente. |

Cerrar un stream o listener hace que las operaciones posteriores devuelvan `Err(NetError)` con `ERR_NET_CLOSED` e interrumpe lecturas o accepts pendientes sin bloquear el executor. Los constructores TCP y los métodos de los handles devuelven `Result<T, NetError>` para fallos recuperables; los tiempos de espera y los demás fallos llevan códigos `ERR_NET_*` dedicados. `stream()` entrega la conexión a un `DuplexStream<u8[], u8[]>` cuyos lados aplican backpressure; cerrar su lado de escritura cierra solo la dirección de escritura, así que el otro extremo lee fin de archivo mientras este lado aún puede leer la respuesta. Desde entonces el duplex es dueño del socket, y `read`, `write` y `shutdown` del propio stream devuelven `Err(NetError)` con `ERR_NET_STREAMED`.

## UDP

`udpBind(address)` devuelve `Promise<Result<UdpSocket, NetError>>`. `sendTo` envía un datagrama y devuelve la cantidad de bytes enviados. `receiveFrom` devuelve los bytes recibidos y la dirección del remitente; un datagrama mayor que `maxBytes` se trunca a ese límite. El payload máximo de un datagrama es de 65.507 bytes. Todos los fallos UDP recuperables cumplen con `Err(NetError)`.

| Elemento | Firma | Finalidad |
|------|------------|---------|
| `UdpSocket.sendTo` | `(data: u8[], address: string, timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Envía un datagrama a la dirección de destino. |
| `UdpSocket.receiveFrom` | `(maxBytes: u32, timeoutMs?: u64) => Promise<Result<UdpPacket, NetError>>` | Recibe un datagrama y lo trunca a `maxBytes`. |
| `UdpSocket.setBroadcast` | `(enabled: boolean) => Promise<Result<void, NetError>>` | Activa o desactiva el envío por broadcast. |
| `UdpSocket.setTtl` | `(ttl: u32) => Promise<Result<void, NetError>>` | Establece el valor de time-to-live de IP. |
| `UdpSocket.localAddress` | `() => string` | Devuelve la dirección local del socket. |
| `UdpSocket.close` | `() => void` | Cierra el socket y despierta un receive pendiente. |
| `UdpPacket.data` | `u8[]` | Contiene el payload del datagrama. |
| `UdpPacket.address` | `string` | Contiene la dirección del socket remitente. |

Cerrar un socket UDP hace que las operaciones posteriores devuelvan `Err(NetError)` con `ERR_NET_CLOSED`. Un tiempo de espera devuelve `ERR_NET_TIMEOUT`; las entradas inválidas y los fallos de bind, asignación, opciones o recepción usan códigos `ERR_NET_*` dedicados. TTL acepta valores de 1 a 255 y devuelve `ERR_NET_INVALID_TTL` para los demás. Los cambios de broadcast y TTL devuelven `ERR_NET_PERMISSION` cuando el sistema operativo deniega la operación y `ERR_NET_OPTION` para otros fallos de la plataforma.

## Sondeos de red

`supportsIcmp()` informa si la plataforma puede abrir el identificador o los sockets nativos de sondeo ICMP. `ping` envía una solicitud ICMP Echo. `traceroute` informa los saltos IPv4 que responden, usando "*" cuando un salto no responde antes del timeout; Linux envía sondeos UDP con TTL creciente, mientras que Windows envía sondeos ICMP Echo con TTL creciente. macOS devuelve `false` y los sondeos cumplen con `Err(NetError)` usando `ERR_NET_UNSUPPORTED`; los sondeos nunca sustituyen ICMP por una conexión TCP.

| Elemento | Firma | Finalidad |
|------|------------|---------|
| [`supportsIcmp`](./supports-icmp.md) | `() => boolean` | Informa si los sondeos ICMP están disponibles. |
| [`ping`](./ping.md) | `(address: string, timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Mide un sondeo ICMP y devuelve el tiempo de ida y vuelta en milisegundos cuando hay soporte. |
| [`traceroute`](./traceroute.md) | `(address: string, timeoutMs?: u64) => Promise<Result<string[], NetError>>` | Informa hasta 30 saltos IPv4 cuando hay soporte. |

## Ejemplo

<!-- @include: ../../../.fragments/modules/net/index/example.md -->
