```tyrus
import { NetworkInterface, OsError, networkInterfaces } from "tyrus:os";

const result: Result<NetworkInterface[], OsError> = networkInterfaces();
match (result) {
    Ok(interfaces) => { print(`${interfaces.length} address(es)`); }
    Err(error) => { printError(error.code); }
}
```
