```tyrus
import { readFile, FsError } from "tyrus:fs";

const path: string = "config.json";
match (await readFile(path)) {
    Ok(contents) => { print(contents); } // { ... }
    Err(error) => { printError(error.code); } // ERR_FS_READ_FAILED
}
```
