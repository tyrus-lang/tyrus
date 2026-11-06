```tyrus
import { HttpRequest, HttpResponse, HttpStatus } from "tyrus:http";

function requireDocsAdmin(request: HttpRequest): HttpResponse | true {
    if (isAuthorizedAdmin(request)) {
        return true;
    }
    return { statusCode: HttpStatus.Forbidden, body: "", cookies: "" };
}

app.docs({ preHandler: [requireDocsAdmin] });
```
