```tyrus
import { OsError, allowSleep, preventSleep } from "tyrus:os";

match (preventSleep()) {
    Ok() => print("Sleep prevention enabled"); // Sleep prevention enabled
    Err(error) => printError(`${error.code}: ${error.message}`); // ERR_OS_SLEEP_INHIBIT_FAILED: platform sleep prevention is unavailable
}

match (allowSleep()) {
    Ok() => print("Sleep prevention disabled"); // Sleep prevention disabled
    Err(error) => printError(`${error.code}: ${error.message}`); // ERR_OS_SLEEP_ALLOW_FAILED: platform sleep prevention could not be released
}
```
