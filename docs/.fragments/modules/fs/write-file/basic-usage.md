```tyrus
import { writeFile, FsError } from "tyrus:fs";

match (await writeFile("message.txt", "hello from tyrus")) {
    Ok(_) => { print("write complete"); } // write complete
    Err(error) => { printError(error.code); } // ERR_FS_WRITE_FAILED
}
```
