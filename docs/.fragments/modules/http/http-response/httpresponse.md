```tyrus
import { HttpRequest, HttpResponse, HttpStatus } from "tyrus:http";

export function requireAuthorization(request: HttpRequest): HttpResponse | true {
    if (request.headers.includes("authorization")) {
        return true;
    }
    return { statusCode: HttpStatus.Unauthorized, body: "Authorization required", contentType: "text/plain; charset=utf-8", cookies: "" };
}
```
