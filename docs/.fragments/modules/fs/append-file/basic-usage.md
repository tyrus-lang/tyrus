```tyrus
import { appendFile, FsError } from "tyrus:fs";

match (await appendFile("log.txt", "new entry\n")) {
    Ok(_) => { print("append complete"); } // append complete
    Err(error) => { printError(error.code); } // ERR_FS_APPEND_FAILED
}
```
