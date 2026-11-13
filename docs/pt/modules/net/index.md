---
title: tyrus:net
description: APIs nativas de endereços, resolução de nomes, TCP e UDP assíncronos e sondagens de rede condicionais à capacidade.
---

# tyrus:net

`tyrus:net` oferece parsing validado de endereços, resolução de nomes do sistema, comunicação TCP e UDP assíncrona e sondagens de rede condicionais à capacidade para programas nativos.

## Imports

<SignatureBlock label="Importações">import { NetError, TcpListener, TcpStream, UdpPacket, UdpSocket, tcpConnect, tcpListen, udpBind, ping, traceroute, supportsIcmp, parseIpAddress, parseSocketAddress, resolve, reverseLookup } from "tyrus:net";</SignatureBlock>

## Parsing e resolução de endereços

| Item | Assinatura | Finalidade |
|------|------------|------------|
| [`parseIpAddress`](./parse-ip-address.md) | `(value: string) => IpAddress?` | Faz parsing de um endereço IPv4 ou IPv6 sem realizar I/O. |
| [`parseSocketAddress`](./parse-socket-address.md) | `(value: string) => SocketAddress?` | Faz parsing de um endereço com porta, incluindo a notação IPv6 entre colchetes. |
| [`resolve`](./resolve.md) | `(host: string, timeoutMs?: u64) => Promise<Result<string[], NetError>>` | Resolve um host pelo resolvedor do sistema. |
| [`reverseLookup`](./reverse-lookup.md) | `(address: string, timeoutMs?: u64) => Promise<Result<string, NetError>>` | Realiza resolução reversa de nome para um endereço. |

Texto de endereço inválido retorna um opcional ausente nas funções de parsing. `resolve` e `reverseLookup` cumprem com `Err(NetError)` quando a resolução falha, e `ping` e `traceroute` fazem o mesmo em falhas recuperáveis das sondagens; o `code` estável identifica a falha e a `message` é legível.

## Contrato e suporte

Os handles de rede são tipos importados explícitos. Os métodos emprestam o handle, portanto o socket continua utilizável até `close()`; fechar um handle TCP interrompe I/O pendente e as operações posteriores retornam `Err(NetError)` com `ERR_NET_CLOSED`, e as falhas UDP também retornam `Err(NetError)`. Operações TCP e UDP podem ocorrer concorrentemente, mas cada operação do socket é serializada pelo handle. `NetworkInterface` continua sendo fornecida por `tyrus:os` para evitar um tipo duplicado de interface de rede.

| Código | Significado |
|------|---------|
| `ERR_NET_INVALID_ADDRESS` | O texto do endereço é inválido. |
| `ERR_NET_TIMEOUT` | A operação excedeu seu timeout. |
| `ERR_NET_PERMISSION` | O sistema operacional ou sandbox negou a operação. |
| `ERR_NET_CLOSED` | O socket foi fechado. |
| `ERR_NET_UNSUPPORTED` | A plataforma atual não possui implementação para a operação. |

A matriz inicial de suporte é Linux e Windows para parsing de endereços, resolução, TCP, UDP, ping e traceroute; macOS suporta parsing de endereços, resolução, TCP e UDP, enquanto sondagens ICMP retornam `Err(NetError)` com `ERR_NET_UNSUPPORTED`.

## TCP

`tcpConnect(address, timeoutMs?)` retorna `Promise<Result<TcpStream, NetError>>`. `tcpListen(address, timeoutMs?)` retorna `Promise<Result<TcpListener, NetError>>`. O timeout opcional é expresso em milissegundos.

| Item | Assinatura | Finalidade |
|------|------------|------------|
| `TcpStream.read` | `(maxBytes: u32, timeoutMs?: u64) => Promise<Result<u8[], NetError>>` | Lê até a quantidade solicitada de bytes; EOF é um array vazio. |
| `TcpStream.write` | `(data: u8[], timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Escreve bytes e retorna a quantidade escrita, que pode ser menor que a entrada. |
| `TcpStream.shutdown` | `() => Promise<Result<void, NetError>>` | Desliga o lado de escrita do stream. |
| `TcpStream.localAddress` | `() => string` | Retorna o endereço local do socket. |
| `TcpStream.peerAddress` | `() => string` | Retorna o endereço remoto do socket. |
| `TcpStream.stream` | `() => Promise<Result<DuplexStream<u8[], u8[]>, NetError>>` | Entrega a conexão a um duplex de bytes limitado. |
| `TcpStream.close` | `() => void` | Fecha o stream e acorda I/O pendente. |
| `TcpListener.accept` | `(timeoutMs?: u64) => Promise<Result<TcpStream, NetError>>` | Aceita a próxima conexão. |
| `TcpListener.localAddress` | `() => string` | Retorna o endereço do socket em escuta. |
| `TcpListener.close` | `() => void` | Fecha o listener e acorda um accept pendente. |

Fechar um stream ou listener faz as operações posteriores retornarem `Err(NetError)` com `ERR_NET_CLOSED` e interrompe leituras ou accepts pendentes sem bloquear o executor. Os construtores TCP e os métodos dos handles retornam `Result<T, NetError>` para falhas recuperáveis; timeouts e demais falhas carregam códigos `ERR_NET_*` dedicados. `stream()` entrega a conexão a um `DuplexStream<u8[], u8[]>` cujos lados aplicam backpressure; fechar o lado de escrita encerra só a direção de escrita, então o outro lado lê fim de arquivo enquanto este ainda pode ler a resposta. A partir daí o duplex é dono do socket, e `read`, `write` e `shutdown` do próprio stream retornam `Err(NetError)` com `ERR_NET_STREAMED`.

## UDP

`udpBind(address)` retorna `Promise<Result<UdpSocket, NetError>>`. `sendTo` envia um datagrama e retorna a quantidade de bytes enviados. `receiveFrom` retorna os bytes recebidos e o endereço do remetente; um datagrama maior que `maxBytes` é truncado nesse limite. O payload máximo de um datagrama é de 65.507 bytes. Todas as falhas recuperáveis de UDP cumprem com `Err(NetError)`.

| Item | Assinatura | Finalidade |
|------|------------|------------|
| `UdpSocket.sendTo` | `(data: u8[], address: string, timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Envia um datagrama ao endereço de destino. |
| `UdpSocket.receiveFrom` | `(maxBytes: u32, timeoutMs?: u64) => Promise<Result<UdpPacket, NetError>>` | Recebe um datagrama, truncando-o em `maxBytes`. |
| `UdpSocket.setBroadcast` | `(enabled: boolean) => Promise<Result<void, NetError>>` | Habilita ou desabilita o envio por broadcast. |
| `UdpSocket.setTtl` | `(ttl: u32) => Promise<Result<void, NetError>>` | Define o valor de time-to-live do IP. |
| `UdpSocket.localAddress` | `() => string` | Retorna o endereço associado ao socket. |
| `UdpSocket.close` | `() => void` | Fecha o socket e acorda um receive pendente. |
| `UdpPacket.data` | `u8[]` | Contém o payload do datagrama. |
| `UdpPacket.address` | `string` | Contém o endereço do socket remetente. |

Fechar um socket UDP faz as operações seguintes retornarem `Err(NetError)` com `ERR_NET_CLOSED`. Um timeout retorna `ERR_NET_TIMEOUT`; entrada inválida, falhas de bind, alocação, opções e recebimento usam códigos `ERR_NET_*` dedicados. TTL aceita valores de 1 a 255 e retorna `ERR_NET_INVALID_TTL` para os demais. Alterações de broadcast e TTL retornam `ERR_NET_PERMISSION` quando o sistema operacional negar a operação e `ERR_NET_OPTION` para outras falhas da plataforma.

## Sondagens de rede

`supportsIcmp()` informa se a plataforma consegue abrir o identificador ou os sockets nativos de sondagem ICMP. `ping` envia uma requisição ICMP Echo. `traceroute` informa os saltos IPv4 que respondem, usando "*" quando um salto não responde até o timeout; Linux envia sondagens UDP com TTL crescente, enquanto Windows envia sondagens ICMP Echo com TTL crescente. macOS retorna `false` e as sondagens cumprem com `Err(NetError)` usando `ERR_NET_UNSUPPORTED`; as sondagens nunca substituem ICMP por uma conexão TCP.

| Item | Assinatura | Finalidade |
|------|------------|------------|
| [`supportsIcmp`](./supports-icmp.md) | `() => boolean` | Informa se sondagens ICMP estão disponíveis. |
| [`ping`](./ping.md) | `(address: string, timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Mede uma sondagem ICMP e retorna o tempo de ida e volta em milissegundos quando há suporte. |
| [`traceroute`](./traceroute.md) | `(address: string, timeoutMs?: u64) => Promise<Result<string[], NetError>>` | Informa até 30 saltos IPv4 quando há suporte. |

## Exemplo

<!-- @include: ../../../.fragments/modules/net/index/example.md -->
