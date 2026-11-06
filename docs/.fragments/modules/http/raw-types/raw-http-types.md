```tyrus
import { CorsOptions, createRouter, HttpError, HttpReply, HttpRequest, HttpResponse, HttpRouter, HttpStatus } from "tyrus:http";

interface HealthResponse {
    ok: boolean;
}

function requireAuthorization(request: HttpRequest): HttpResponse | true {
    if (request.headers.includes("authorization")) {
        return true;
    }
    return { statusCode: HttpStatus.Unauthorized, body: "Authorization required", cookies: "" };
}

const cors: CorsOptions = {
    origin: ["https://example.com"],
    methods: ["GET"],
    allowedHeaders: ["Authorization"],
    exposedHeaders: [],
    credentials: false,
    maxAge: 3600,
};

const app: HttpRouter = createRouter();
app.cors(cors);
app.get("/health", {
    response: { 200: HealthResponse },
    preHandler: [requireAuthorization],
    handler: (request, reply: HttpReply<HealthResponse>) => {
        reply.status(HttpStatus.Ok).send({ ok: true });
    },
});

try {
    app.listen(3333);
} catch (error: HttpError) {
    console.error(`${error.code}: ${error.message}`);
}
```
