```tyrus
import { removeAll, type FsError } from "tyrus:fs";

match (removeAll("tempdir")) {
    Ok(_) => {}
    Err(error) => { printError(error.code); // ERR_FS_REMOVE_FAILED
    }
}
```
