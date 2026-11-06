```tyrus
import { exists, readFileSync, type FsError } from "tyrus:fs";

const main = (): void => {
    const path: string = "config.json";
    if (exists(path)) {
        match (readFileSync(path)) {
            Ok(config) => print(config);
            Err(error) => printError(error.message);
        }
    }
};
```
