```tyrus
import { HttpAccessLog, HttpStatus } from "tyrus:http";
import { Duration } from "tyrus:temporal";

function recordAccess(access: HttpAccessLog): void {
    const duration: Duration = access.duration;
    console.error(access.requestId, access.status);
}
```
