```tyrus
import { FsError, readFile, writeFile } from "tyrus:fs";

match (await readFile("README.md")) {
    Ok(source) => {
        match (await writeFile("target/readme-copy.md", source)) {
            Ok(_) => { print("copy complete"); } // copy complete
            Err(error) => { printError(error.code); } // ERR_FS_WRITE_FAILED
        }
    }
    Err(error) => { printError(error.code); } // ERR_FS_READ_FAILED
}
```
