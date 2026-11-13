```tyrus
import { OsError, openUrl } from "tyrus:os";

match (openUrl("https://example.com")) {
    Ok() => print("URL opened"); // URL opened
    Err(error) => printError(`${error.code}: ${error.message}`); // ERR_OS_NO_HANDLER: no application is registered to open this URL
}
```
