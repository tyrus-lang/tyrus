```tyrus
import { stat, type FileStats, type FsError } from "tyrus:fs";

const result: Result<FileStats, FsError> = stat("./message.txt");
match (result) {
    Ok(info) => {
        print(info.size); // 18
        print(info.isFile); // true
        print(info.isDirectory); // false
    },
    Err(_) => {},
}
```
