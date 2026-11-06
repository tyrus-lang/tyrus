```tyrus
import { FsError, symlinkDir } from "tyrus:fs";

match (symlinkDir("./original-directory", "./directory-shortcut")) {
    Ok(_) => { print("directory symlink created"); } // directory symlink created
    Err(error: FsError) => { printError(error.message); }
}
```
