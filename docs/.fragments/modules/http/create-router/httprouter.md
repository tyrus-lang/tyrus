```tyrus
import { CorsOptions, createRouter, HttpError, HttpReply, HttpRouter, HttpStatus } from "tyrus:http";

export function buildRouter(): Result<HttpRouter, HttpError> {
    const app: HttpRouter = createRouter();
    const cors: CorsOptions = {
        origin: ["https://example.com"],
        methods: ["GET"],
        allowedHeaders: [],
        exposedHeaders: [],
        credentials: false,
        maxAge: 0,
    };

    app.cors(cors)?;
    app.get("/health", {
        response: { 200: string },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send("ok");
        }
    })?;

    return Ok(app);
}
```
