```tyrus
import { NetError, UdpPacket, UdpSocket, udpBind } from "tyrus:net";

async function receive(): Promise<void> {
    const bindResult: Result<UdpSocket, NetError> = await udpBind("127.0.0.1:0");
    match (bindResult) {
        Ok(socket) => {
            const result: Result<UdpPacket, NetError> = await socket.receiveFrom(1024, 1000);
            match (result) {
                Ok(packet) => { print(packet.data.length, packet.address); // 5, "127.0.0.1:50000"
                }
                Err(error) => { printError(error.code); }
            }
            socket.close();
        }
        Err(error) => { printError(error.code); }
    }
}
```
