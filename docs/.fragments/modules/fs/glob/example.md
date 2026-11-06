```tyrus
import { FsError, glob } from "tyrus:fs";

match (glob("*.txt")) {
    Ok(files) => { print(files.length); } // 2
    Err(error) => { printError(error.code); } // ERR_FS_GLOB_FAILED
}

match (glob("./src/**/*.ts")) {
    Ok(pyFiles) => { print(pyFiles.length); } // 4
    Err(error) => { printError(error.code); } // ERR_FS_GLOB_FAILED
}
```
