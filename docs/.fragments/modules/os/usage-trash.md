```tyrus
import { OsError, trash } from "tyrus:os";

match (trash("./stale-report.txt")) {
    Ok() => print("File moved to trash"); // File moved to trash
    Err(error) => printError(`${error.code}: ${error.message}`); // ERR_OS_TRASH_FAILED: path does not exist, or the trash service is unavailable
}
```
