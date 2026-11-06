```tyrus
import { createRouter, HttpAccessLog, HttpError, HttpRouter } from "tyrus:http";
import { Duration } from "tyrus:temporal";

function recordAccess(access: HttpAccessLog): void {
    const elapsed: Duration = access.duration;
    console.log(access.requestId, access.status);
    console.log("assert:", access.status);
}

const app: HttpRouter = createRouter();
app.onResponse(recordAccess);

try {
    app.listen(3000);
} catch (error: HttpError) {
    console.error(error.code);
}
```
