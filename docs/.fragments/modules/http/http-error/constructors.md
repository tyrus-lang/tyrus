```tyrus
import { HttpError, HttpStatus } from "tyrus:http";

return reply.error(HttpError.NotFound("User not found"));
```
