```tyrus
import { NetError, supportsIcmp, traceroute } from "tyrus:net";

async function trace(): Promise<void> {
    if (supportsIcmp()) {
        const result: Result<string[], NetError> = await traceroute("127.0.0.1", 1000);
        match (result) {
            Ok(hops) => { print(hops.length); } // 1
            Err(error) => { printError(error.code); } // ERR_NET_TIMEOUT
        }
    }
}
```
