```tyrus
import { FsError, tempFile } from "tyrus:fs";

match (tempFile()) {
    Ok(path) => { print(path); } // /tmp/tyrus_abc123.tmp
    Err(error) => { printError(error.code); } // ERR_FS_TEMP_FILE_FAILED
}
```
