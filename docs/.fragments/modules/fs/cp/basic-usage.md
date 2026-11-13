```tyrus
import { cp, type FsError } from "tyrus:fs";

match (cp("source.txt", "dest.txt")) {
    Ok(_) => {}
    Err(error) => { printError(error.code); // ERR_FS_CP_FAILED
    }
}
```
