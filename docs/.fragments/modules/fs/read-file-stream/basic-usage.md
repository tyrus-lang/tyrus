```tyrus
import { FsError, readFileStream } from "tyrus:fs";
import { ReadableStream } from "tyrus:stream";

match (readFileStream("video.mp4")) {
    Ok(stream) => { print("input stream opened"); } // input stream opened
    Err(error) => { printError(error.code); } // ERR_FS_READ_FAILED
}
```
