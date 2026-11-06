```tyrus
const cors: CorsOptions = {
    origin: ["*"], // [!code error]
    methods: ["GET"],
    allowedHeaders: [],
    exposedHeaders: [],
    credentials: true,
    maxAge: 0,
};
```
