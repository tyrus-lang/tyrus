```tyrus
import { FsError, readdir } from "tyrus:fs";

match (readdir(".")) {
    Ok(entries) => { print(entries.length); } // 3
    Err(error) => { printError(error.code); } // ERR_FS_READ_FAILED
}
```
