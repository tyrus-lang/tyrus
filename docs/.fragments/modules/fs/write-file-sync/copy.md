```tyrus
import { readFileSync, writeFileSync, type FsError } from "tyrus:fs";

const main = (): void => {
    match (readFileSync("README.md")) {
        Ok(source) => {
            match (writeFileSync("target/readme-copy.md", source)) {
                Ok() => print("file copied");
                Err(error) => printError(error.message);
            }
        }
        Err(error) => printError(error.message);
    }
};
```
