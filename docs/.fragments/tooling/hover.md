```tyrus
import { readFileSync, type FsError } from "tyrus:fs";

const main = (): void => {
    match (readFileSync("README.md")) {
        Ok(contents) => print(contents);
        Err(error) => printError(error.code);
    }
};
```
