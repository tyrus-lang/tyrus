```tyrus
import { OsError, clipboardRead } from "tyrus:os";

function readClipboard(): Result<string?, OsError> {
    return clipboardRead();
}
```
