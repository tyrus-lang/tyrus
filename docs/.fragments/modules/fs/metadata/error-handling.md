```tyrus
import { FsError, metadata } from "tyrus:fs";

match (metadata("/nonexistent")) {
    Ok(info) => { print(info.size); }
    Err(error) => { printError(error.code); } // ERR_FS_METADATA_FAILED
}
```
