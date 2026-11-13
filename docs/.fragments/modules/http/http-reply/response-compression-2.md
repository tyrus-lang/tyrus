```tyrus
import { HttpReply, HttpStatus } from "tyrus:http";

interface ApiError {
    code: string;
    message: string;
}

export function userNotFound(reply: HttpReply<ApiError>): void {
    reply.status(HttpStatus.NotFound).send({
        code: "user_not_found",
        message: "User not found",
    });
}
```
