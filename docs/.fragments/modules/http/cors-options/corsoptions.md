```tyrus
import { CorsOptions, createRouter, HttpRouter } from "tyrus:http";

const options: CorsOptions = {
    origin: ["https://app.example.com"],
    methods: ["GET", "POST", "OPTIONS", "QUERY"],
    allowedHeaders: ["Content-Type", "Authorization"],
    exposedHeaders: ["x-request-id"],
    credentials: true,
    maxAge: 3600,
};

const app: HttpRouter = createRouter();
app.cors(options);
```
