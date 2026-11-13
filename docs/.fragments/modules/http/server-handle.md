```tyrus
import { createRouter, HttpError, HttpRouter, HttpServer } from "tyrus:http";

const main = async (): Promise<void> => {
    const app: HttpRouter = createRouter();
    const server = app.listen(3000, { shutdownTimeout: "10s" }) onfail {
        printError(`${server.code}: ${server.message}`);
    };
    print(`Server listening on port ${server.port()}`);
    await server.close();
};
```
