```tyrus
import { readFile, FsError } from "tyrus:fs";

match (await readFile("README.md")) {
    Ok(readme) => { print(readme); } // # project
    Err(error) => { printError(error.code); } // ERR_FS_READ_FAILED
}
```
