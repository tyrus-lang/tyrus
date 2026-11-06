```tyrus
import { createRouter, HttpError, HttpRouter, HttpServer } from "tyrus:http";

const main = (): void => {
    const app: HttpRouter = createRouter();
    const server = app.listen(3000) onfail {
        printError(`${server.code}: ${server.message}`);
    };
    print(`Server listening on port ${server.port()}`);
};
```
