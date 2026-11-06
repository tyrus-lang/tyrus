```tyrus
import { rm, type FsError } from "tyrus:fs";

match (rm("temp.txt")) {
    Ok(_) => {}
    Err(error) => { printError(error.code); // ERR_FS_RM_FAILED
    }
}
match (rm("subdir", { recursive: true, force: true })) {
    Ok(_) => {}
    Err(error) => { printError(error.code); // ERR_FS_RM_FAILED
    }
}
```
