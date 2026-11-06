```tyrus
import { NotifyOptions, NotifyUrgency, OsError, notify } from "tyrus:os";

const main = (): void => {
    match (notify("Build complete", "All 42 tests passed.")) {
        Ok() => print("Notification sent"); // Notification sent
        Err(error) => printError(`${error.code}: ${error.message}`); // ERR_OS_NOTIFICATION_FAILED: the platform notification service could not show the notification
    }

    const options: NotifyOptions = { urgency: NotifyUrgency.Critical, timeout: 5000 };
    match (notify("Build failed", "3 tests failed.", options)) {
        Ok() => print("Critical notification sent"); // Critical notification sent
        Err(error) => printError(`${error.code}: ${error.message}`); // ERR_OS_NOTIFICATION_FAILED: the platform notification service could not show the notification
    }
};
```
