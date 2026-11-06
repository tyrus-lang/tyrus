```tyrus
import { NetError, TcpListener, tcpListen } from "tyrus:net";

async function listen(): Promise<void> {
    const result: Result<TcpListener, NetError> = await tcpListen("", 1000);
    match (result) {
        Ok(listener) => { listener.close(); }
        Err(error) => { printError(error.code); }
    }
}
```
