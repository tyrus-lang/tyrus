```tyrus
import { FsError, search } from "tyrus:fs";

match (search("*.txt", { recursive: true, maxResults: 100 })) {
    Ok(matches) => { print(matches.length); } // 2
    Err(error) => { printError(error.code); } // ERR_FS_SEARCH_FAILED
}
```
