```tyrus
app.get("/health", {
    response: { 200: HealthResponse },
    handler: (request, reply): void => {
        reply.header("x-content-type-options", "nosniff");
        reply.header("referrer-policy", "no-referrer");
        reply.header("x-frame-options", "DENY");
        reply.header("cross-origin-resource-policy", "same-origin");
        reply.status(HttpStatus.Ok).send({ status: "ok" });
    },
});
```
