```tyrus
import { readFileSync, type FsError } from "tyrus:fs";

const main = (): void => {
    const path: string = "config.json";
    match (readFileSync(path)) {
        Ok(contents) => print(contents);
        Err(error) => printError(error.message);
    }
};
```
