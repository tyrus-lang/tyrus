```tyrus
import { FsError, atomicWrite } from "tyrus:fs";

match (atomicWrite("./config.json", '{"version": 2}')) {
    Ok(_) => { print("configuration written"); } // configuration written
    Err(error) => { printError(error.code); } // ERR_FS_WRITE_FAILED
}
```
