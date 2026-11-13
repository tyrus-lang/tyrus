```tyrus
import { createRouter, HttpError, HttpRouter, ServerOptions } from "tyrus:http";

const app: HttpRouter = createRouter();
const options: ServerOptions = {
    host: "127.0.0.1",
    requestTimeout: "30s",
    maxConcurrentRequests: 1024,
    maxConnections: 4096,
    shutdownTimeout: "10s",
    logger: true,
};

try {
    app.listen(3000, options);
} catch (error: HttpError) {
    printError(`${error.code}: ${error.message}`);
}
```
