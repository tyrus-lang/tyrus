```tyrus
import { readFileSync, type FsError } from "tyrus:fs";

const add = (left: i32, right: i32): i32 => left + right;

interface FormatResult {
    value: i32;
    source: Result<string, FsError>;
}

const main = (): void => {
    const result: FormatResult = {
        value: add(1, 2),
        source: readFileSync("README.md"),
    };
};
```
