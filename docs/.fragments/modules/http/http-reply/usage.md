```tyrus
import { HttpReply, HttpStatus } from "tyrus:http";

interface UserResponse {
    id: i64;
    email: string;
}

export function showUser(reply: HttpReply<UserResponse>): void {
    reply.status(HttpStatus.Ok).send({
        id: 1,
        email: "ada@example.com",
    });
}
```
