```tyrus
import { NetError, UdpSocket, udpBind } from "tyrus:net";

async function openSocket(): Promise<Result<void, NetError>> {
    const socket: UdpSocket = (await udpBind("127.0.0.1:0"))?;
    print(socket.localAddress()); // "127.0.0.1:random-port"
    socket.close();
    return Ok();
}
```
