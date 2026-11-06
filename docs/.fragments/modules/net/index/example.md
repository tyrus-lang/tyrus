```tyrus
import { NetError, TcpListener, tcpListen } from "tyrus:net";

async function openListener(): Promise<Result<void, NetError>> {
    const listener: TcpListener = (await tcpListen("127.0.0.1:0", 1000))?;
    print(listener.localAddress()); // "127.0.0.1:random-port"
    listener.close();
    return Ok();
}
```
