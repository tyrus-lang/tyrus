```tyrus
import { NetError, UdpSocket, udpBind } from "tyrus:net";

async function useUdp(): Promise<Result<void, NetError>> {
    const socket: UdpSocket = (await udpBind("127.0.0.1:0"))?;
    (await socket.setBroadcast(false))?;
    (await socket.setTtl(64))?;
    const sent: u64 = (await socket.sendTo([104, 101, 108, 108, 111], "127.0.0.1:9000", 1000))?;
    print(socket.localAddress(), sent); // "127.0.0.1:random-port", 5
    socket.close();
    return Ok();
}
```
