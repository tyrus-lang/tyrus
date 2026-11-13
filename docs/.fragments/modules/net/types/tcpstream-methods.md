```tyrus
import { NetError, TcpStream, tcpConnect } from "tyrus:net";

async function useTcp(): Promise<Result<void, NetError>> {
    const stream: TcpStream = (await tcpConnect("127.0.0.1:8080", 1000))?;
    const written: u64 = (await stream.write([104, 105], 1000))?;
    const data: u8[] = (await stream.read(1024, 1000))?;
    (await stream.shutdown())?;
    print(stream.localAddress(), stream.peerAddress(), written, data.length); // "127.0.0.1:random-port", "127.0.0.1:8080", 2, 2
    stream.close();
    return Ok();
}
```
