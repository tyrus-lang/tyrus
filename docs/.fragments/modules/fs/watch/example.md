```tyrus
import { FsError, watch } from "tyrus:fs";

match (watch("./logs")) {
    Ok(watcher) => {
        print("watching logs"); // watching logs
        watcher.close();
    }
    Err(error) => { printError(error.code); } // ERR_FS_WATCH_FAILED
}
```
