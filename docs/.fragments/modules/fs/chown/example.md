```tyrus
import { FsError, chown } from "tyrus:fs";

// chown requires appropriate privileges
match (chown("./file.txt", 1000, 1000)) {
    Ok(_) => { print("ownership updated"); } // ownership updated
    Err(error) => { printError(error.code); } // ERR_FS_CHOWN_FAILED
}
```
