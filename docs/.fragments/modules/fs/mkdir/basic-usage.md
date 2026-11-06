```tyrus
import { mkdir, type FsError } from "tyrus:fs";

match (mkdir("subdir")) {
    Ok(_) => { print("directory created"); }
    Err(error) => { printError(error.code); // ERR_FS_MKDIR_FAILED
    }
}
```
