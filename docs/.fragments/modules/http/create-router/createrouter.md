```tyrus
import { createRouter, HttpStatus } from "tyrus:http";

interface HealthResponse {
    ok: boolean;
}

const app: HttpRouter = createRouter();

app.get("/health", {
    response: {
        200: HealthResponse
    },
    handler: (request, reply) => {
        reply.status(HttpStatus.Ok).send({ ok: true });
    }
});

app.listen(3333);
```
