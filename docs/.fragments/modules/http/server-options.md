```tyrus
import { createRouter, HttpError, HttpRouter, RateLimit, ServerOptions, TlsOptions } from "tyrus:http";

const app: HttpRouter = createRouter();

try {
    app.listen(3000, {
        host: "127.0.0.1",
        requestTimeout: "30s",
        maxConcurrentRequests: 1024,
        maxConnections: 4096,
        shutdownTimeout: "10s",
        rateLimit: { max: 300, timeWindow: "1 minute" },
        headerTimeout: "10s",
        maxHeaderSize: "32kib",
        keepAlive: true,
        tls: { cert: "certs/server.crt", key: "certs/server.key" },
    });
} catch (error: HttpError) {
    console.error(`${error.code}: ${error.message}`);
}
```
