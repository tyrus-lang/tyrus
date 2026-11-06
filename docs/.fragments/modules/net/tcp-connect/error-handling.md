```tyrus
import { NetError, TcpStream, tcpConnect } from "tyrus:net";

async function connect(): Promise<void> {
    const result: Result<TcpStream, NetError> = await tcpConnect("127.0.0.1:1", 100);
    match (result) {
        Ok(stream) => { stream.close(); }
        Err(error) => { printError(error.code); }
    }
}
```
