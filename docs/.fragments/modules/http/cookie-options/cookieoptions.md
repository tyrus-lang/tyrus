```tyrus
import { CookieOptions, HttpReply, HttpStatus } from "tyrus:http";

interface SessionResponse {
    token: string;
}

const options: CookieOptions = {
    path: "/",
    maxAge: 3600,
    secure: true,
    httpOnly: true,
    sameSite: "lax",
};

export function createSession(reply: HttpReply<SessionResponse>): void {
    reply
        .setCookie("session", "token", options)
        .status(HttpStatus.Created)
        .send({ token: "token" });
}
```
