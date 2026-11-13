```tyrus
import { FsError, readdir } from "tyrus:fs";

match (readdir("/nonexistent")) {
    Ok(entries) => { print(entries.length); }
    Err(error) => { printError(error.code); } // ERR_FS_READ_FAILED
}
```
