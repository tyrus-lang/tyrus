```tyrus
import { CompressError, decompress } from "tyrus:compress";

const main = async (): Promise<void> => {
    match (await decompress([1, 2, 3])) {
        Ok(original) => { print(original.length); }
        Err(error) => {
            const failure: CompressError = error;
            printError(failure.code); // ERR_COMPRESS_UNKNOWN_FORMAT
            printError(failure.message); // the format cannot be detected from the data; pass `format` in the options
        }
    }
};
```
