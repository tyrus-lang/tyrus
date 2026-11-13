```tyrus
import { FsError, chmod, FileMode } from "tyrus:fs";

match (chmod("./file.txt", FileMode.AllRead)) {
    Ok(_) => { print("permissions updated"); } // permissions updated
    Err(error) => { printError(error.code); } // ERR_FS_CHMOD_FAILED
}
match (chmod("./file.txt", FileMode.FullOwner)) {
    Ok(_) => { print("permissions updated"); } // permissions updated
    Err(error) => { printError(error.code); } // ERR_FS_CHMOD_FAILED
}
```
