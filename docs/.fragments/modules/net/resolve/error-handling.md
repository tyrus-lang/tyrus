```tyrus
import { NetError, resolve } from "tyrus:net";

async function lookup(): Promise<void> {
    const result: Result<string[], NetError> = await resolve("invalid.invalid", 1000);
    match (result) {
        Ok(_) => {}
        Err(error) => { printError(error.code); } // ERR_NET_RESOLVE
    }
}
```
