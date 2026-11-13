```tyrus
import { writeFileSync, type FsError } from "tyrus:fs";

match (writeFileSync("message.txt", "hello from tyrus")) {
    Ok(_) => { print("file written"); }
    Err(error) => { printError(error.code); // ERR_FS_WRITE_FAILED
    }
}
```
