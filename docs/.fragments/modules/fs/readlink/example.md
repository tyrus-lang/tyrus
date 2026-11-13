```tyrus
import { FsError, readlink } from "tyrus:fs";

match (readlink("./link")) {
    Ok(target) => {
        print(target); // "/path/to/original"
    }
    Err(error) => {
        printError(error.code); // ERR_FS_READLINK_FAILED
    }
}
```
