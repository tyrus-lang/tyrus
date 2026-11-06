```tyrus
import { FsError, writeFileStream } from "tyrus:fs";
import { WritableStream } from "tyrus:stream";

match (writeFileStream("archive.bin")) {
    Ok(stream) => { print("output stream opened"); } // output stream opened
    Err(error) => { printError(error.code); } // ERR_FS_WRITE_FAILED
}
```
