```tyrus
import { NetError, resolve } from "tyrus:net";

async function lookup(): Promise<void> {
    const result: Result<string[], NetError> = await resolve("localhost", 1000);
    match (result) {
        Ok(addresses) => { print(addresses.length); } // 1
        Err(error) => { printError(error.code); } // ERR_NET_RESOLVE
    }
}
```
