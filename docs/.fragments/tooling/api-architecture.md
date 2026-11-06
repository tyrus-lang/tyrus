```tyrus
import { HttpReply, HttpStatus } from "tyrus:http";

interface CreatedUserResponse {
    ok: boolean;
}

export const createUserController = (request: EmptyRequest, reply: HttpReply<CreatedUserResponse>): void => {
    reply.status(HttpStatus.Created).send({ ok: true });
};
```
