```tyrus
import { NetError, UdpSocket, udpBind } from "tyrus:net";

async function useClosedSocket(): Promise<Result<void, NetError>> {
    const socket: UdpSocket = (await udpBind("127.0.0.1:0"))?;
    socket.close();
    const result: Result<UdpPacket, NetError> = await socket.receiveFrom(32, 1000);
    match (result) {
        Ok(_) => {}
        Err(error) => { print(error.code); // "ERR_NET_CLOSED"
        }
    }
    return Ok();
}
```
