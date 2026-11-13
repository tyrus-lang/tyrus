```tyrus
import { createRouter, HttpReply, HttpRouter, HttpStatus } from "tyrus:http";

interface UserParams {
    id: string;
}

interface UserQuery {
    limit: i32;
    active: boolean;
}

interface UserResponse {
    id: string;
    limit: i32;
}

const app: HttpRouter = createRouter();

app.get("/users/:id", {
    params: UserParams,
    query: UserQuery,
    response: { 200: UserResponse },
    handler: (request, reply: HttpReply<UserResponse>): void => {
        // `/users/123?limit=25&active=true`
        reply.status(HttpStatus.Ok).send({
            id: request.params.id,
            limit: request.query.limit,
        });
    },
});

app.listen(3000);
```
