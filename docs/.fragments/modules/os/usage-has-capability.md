```tyrus
import { SystemCapability, hasCapability } from "tyrus:os";

if (hasCapability(SystemCapability.Notification)) {
    // safe to call notify(), the notification service is reachable
}
```
