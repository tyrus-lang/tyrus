```tyrus
import { createRouter, HttpAccessLog, HttpRouter } from "tyrus:http";

const app: HttpRouter = createRouter();
app.onResponse((access: HttpAccessLog): void => {}); // [!code error]
```
