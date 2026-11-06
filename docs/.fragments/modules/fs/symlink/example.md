```tyrus
import { FsError, symlink } from "tyrus:fs";

match (symlink("./original.txt", "./shortcut.txt")) {
    Ok(_) => { print("symlink created"); } // symlink created
    Err(error) => { printError(error.code); } // ERR_FS_SYMLINK_FAILED
}
```
