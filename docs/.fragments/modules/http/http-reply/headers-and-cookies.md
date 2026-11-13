```tyrus
import { CookieOptions, HttpReply, HttpStatus } from "tyrus:http";

interface UserResponse {
    id: i64;
}

export function listUsers(reply: HttpReply<UserResponse>): void {
    const cookie: CookieOptions = {
        path: "/",
        secure: true,
        httpOnly: true,
        sameSite: "lax",
    };
    reply
        .header("x-request-id", "request-42")
        .appendHeader("link", "</users?page=2>; rel=next")
        .setCookie("session", "token", cookie)
        .type("application/json; charset=utf-8")
        .status(HttpStatus.Ok)
        .send({ id: 1 });
}
```
