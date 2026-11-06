```tyrus
import { createRouter, HttpError, HttpRouter, HttpStatus } from "tyrus:http";

interface ApiError {
    code: string;
    message: string;
}

const app: HttpRouter = createRouter();

app.get("/items/:id", {
    response: { 404: ApiError },
    handler: (request, reply) => {
        const itemId: i64 = request.params.id;

        if (itemId > 100) {
            reply.status(HttpStatus.NotFound).send({ code: "item_not_found", message: "Item not found" });
            return;
        }
    },
});

app.get("/items/:id/custom-error", {
    response: { 404: ApiError },
    handler: (request, reply) => {
        reply.status(HttpStatus.NotFound).send({ code: "item_missing", message: "Item not found" });
    },
});

try {
    app.listen(3000);
} catch (error: HttpError) {
    printError(`${error.code}: ${error.message}`);
}
```
