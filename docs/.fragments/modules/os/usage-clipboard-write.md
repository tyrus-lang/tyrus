```tyrus
import { OsError, clipboardWrite } from "tyrus:os";

const main = (): void => {
    match (clipboardWrite("Copied from a tyrus program!")) {
        Ok() => print("Clipboard text copied"); // Clipboard text copied
        Err(error) => printError(error.code); // ERR_OS_CLIPBOARD_UNAVAILABLE
    }
};
```
