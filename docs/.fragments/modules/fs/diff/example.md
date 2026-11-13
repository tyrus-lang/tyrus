```tyrus
import { diff, FsError } from "tyrus:fs";

match (diff("./dir1", "./dir2")) {
    Ok(result) => { print(result.added.length); } // 1
    Err(error) => { printError(error.code); } // ERR_FS_DIFF_FAILED
}
```
