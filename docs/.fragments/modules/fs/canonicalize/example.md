```tyrus
import { canonicalize, FsError } from "tyrus:fs";

match (canonicalize("./file.txt")) {
    Ok(path) => { print(path); } // /home/user/project/file.txt
    Err(error) => { printError(error.code); } // ERR_FS_CANONICALIZE_FAILED
}
```
