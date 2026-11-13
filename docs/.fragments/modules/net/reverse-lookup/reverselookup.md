```tyrus
import { NetError, reverseLookup } from "tyrus:net";

async function lookup(): Promise<void> {
    const result: Result<string, NetError> = await reverseLookup("127.0.0.1", 1000);
    match (result) {
        Ok(name) => { print(name); } // localhost
        Err(error) => { printError(error.code); } // ERR_NET_REVERSE_LOOKUP
    }
}
```
