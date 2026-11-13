```tyrus
import { NetError, reverseLookup } from "tyrus:net";

async function lookup(): Promise<void> {
    const result: Result<string, NetError> = await reverseLookup("not-an-address", 1000);
    match (result) {
        Ok(_) => {}
        Err(error) => { printError(error.code); } // ERR_NET_INVALID_ADDRESS
    }
}
```
