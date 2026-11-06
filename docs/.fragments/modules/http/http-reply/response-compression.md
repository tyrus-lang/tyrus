```tyrus
import { HttpCacheOptions, HttpReply, HttpReplyOptions, HttpStatus } from "tyrus:http";
import { Instant, instantFrom } from "tyrus:temporal";

interface UserResponse {
    id: i64;
}

const modified: Instant = instantFrom("2026-09-03T12:00:00Z");
const cache: HttpCacheOptions = { etag: "users-v1", lastModified: modified, control: "private, max-age=60" };
const options: HttpReplyOptions = { cache };

reply.status(HttpStatus.Ok).send({ id: 1 }, options);
```
