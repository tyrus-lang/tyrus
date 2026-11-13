```tyrus
import { mv, type FsError } from "tyrus:fs";

match (mv("old.txt", "new.txt")) {
    Ok(_) => {}
    Err(error) => { printError(error.code); // ERR_FS_MV_FAILED
    }
}
```
