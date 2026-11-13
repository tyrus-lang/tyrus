```tyrus
import { NetError, udpBind } from "tyrus:net";

async function openSocket(): Promise<void> {
    const result: Result<UdpSocket, NetError> = await udpBind("");
    match (result) {
        Ok(socket) => { socket.close(); }
        Err(error) => { print(error.code); // "ERR_NET_INVALID_ADDRESS"
        }
    }
}
```
