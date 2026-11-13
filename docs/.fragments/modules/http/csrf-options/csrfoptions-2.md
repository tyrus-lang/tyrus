```tyrus
import { CsrfOptions, createRouter, HttpRouter } from "tyrus:http";

const app: HttpRouter = createRouter();
const csrf: CsrfOptions = { cookie: "__Host-csrf", header: "x csrf token" }; // [!code error]
app.csrf(csrf);
```
