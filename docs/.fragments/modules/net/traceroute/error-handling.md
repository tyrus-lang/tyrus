```tyrus
import { NetError, traceroute } from "tyrus:net";

async function trace(): Promise<void> {
    const result: Result<string[], NetError> = await traceroute("invalid host\n", 1000);
    match (result) {
        Ok(_) => {}
        Err(error) => { printError(error.code); } // ERR_NET_RESOLVE
    }
}
```
