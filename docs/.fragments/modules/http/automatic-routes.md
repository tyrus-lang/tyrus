### `src/routes/health.tyr`

```typescript
import { HttpReply, HttpRouter, HttpStatus } from "tyrus:http";

interface HealthResponse {
    status: string;
}

export const healthRoutes = (router: HttpRouter): void => {
    router.get("/health", {
        response: { 200: HealthResponse },
        handler: (_request: {}, reply: HttpReply<HealthResponse>): void => {
            reply.status(HttpStatus.Ok).send({ status: "ok" });
        },
    });
};
```

### `src/main.tyr`

```typescript
import { createRouter, HttpRouter } from "tyrus:http";

const app: HttpRouter = createRouter();
app.listen(3000);
```
