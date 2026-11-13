```tyrus
import { readFileSync, type FsError } from "tyrus:fs";

match (readFileSync("README.md")) {
    Ok(readme) => { print(readme); }
    Err(error) => { printError(error.code); // ERR_FS_READ_FAILED
    }
}
```
