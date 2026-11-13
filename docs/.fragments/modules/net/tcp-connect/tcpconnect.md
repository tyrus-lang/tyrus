```tyrus
import { NetError, TcpStream, tcpConnect } from "tyrus:net";

async function connect(): Promise<Result<void, NetError>> {
    const stream: TcpStream = (await tcpConnect("127.0.0.1:8080", 1000))?;
    stream.close();
    return Ok();
}
```
