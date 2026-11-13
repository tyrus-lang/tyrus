```tyrus
import { createRouter, HttpRouter, HttpStatus, StreamBody } from "tyrus:http";
import { WritableStream } from "tyrus:stream";

const app: HttpRouter = createRouter();

app.get("/events", {
    response: { 200: StreamBody },
    handler: async (request, reply): Promise<void> => {
        const body: WritableStream<u8[]> = reply.type("application/octet-stream").status(HttpStatus.Ok).stream();
        await body.write([1, 2, 3]);
        await body.close();
    },
});
```
