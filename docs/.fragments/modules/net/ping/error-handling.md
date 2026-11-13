```tyrus
import { NetError, ping } from "tyrus:net";

async function measure(): Promise<void> {
    const result: Result<u64, NetError> = await ping("invalid host\n", 1000);
    match (result) {
        Ok(_) => {}
        Err(error) => { printError(error.code); } // ERR_NET_RESOLVE
    }
}
```
