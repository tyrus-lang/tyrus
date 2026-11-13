```tyrus
import { FsError, hardLink } from "tyrus:fs";

match (hardLink("./original.txt", "./hardlink.txt")) {
    Ok(_) => { print("hard link created"); } // hard link created
    Err(error) => { printError(error.code); } // ERR_FS_HARDLINK_FAILED
}
```
