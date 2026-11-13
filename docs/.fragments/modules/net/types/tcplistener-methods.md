```tyrus
import { NetError, TcpListener, TcpStream, tcpListen } from "tyrus:net";

async function acceptOne(): Promise<Result<void, NetError>> {
    const listener: TcpListener = (await tcpListen("127.0.0.1:0", 1000))?;
    print(listener.localAddress()); // "127.0.0.1:random-port"
    const stream: TcpStream = (await listener.accept(1000))?;
    print(stream.peerAddress()); // "127.0.0.1:random-port"
    stream.close();
    listener.close();
    return Ok();
}
```
