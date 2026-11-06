```tyrus
import { createRouter, HttpRouter } from "tyrus:http";

const app: HttpRouter = createRouter();

app.trustedProxies(["10.0.0.0/8", "::1/128"]);
```
