```tyrus
import { NetError, ping, supportsIcmp } from "tyrus:net";

async function measure(): Promise<void> {
    if (supportsIcmp()) {
        const result: Result<u64, NetError> = await ping("127.0.0.1", 1000);
        match (result) {
            Ok(milliseconds) => { print(milliseconds); } // 0
            Err(error) => { printError(error.code); } // ERR_NET_TIMEOUT
        }
    }
}
```
