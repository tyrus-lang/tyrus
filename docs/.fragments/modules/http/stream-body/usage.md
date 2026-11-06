```tyrus
import { createRouter, HttpReply, HttpRouter, HttpStatus, StreamBody } from "tyrus:http";
import { ReadableStream } from "tyrus:stream";

const app: HttpRouter = createRouter();

app.post("/relay", {
    body: StreamBody,
    response: { 200: StreamBody },
    handler: (request, reply) => {
        reply.status(HttpStatus.Ok).send(request.body);
    },
});
```
