```tyrus
app.docs({
    path: "/reference",
    title: "SQLite Users API",
    version: "0.1.0",
    description: "User endpoints backed by SQLite.",
    auth: { bearer: true, bearerFormat: "JWT" },
    css: ".swagger-ui .topbar { background: #16324f; }",
    preHandler: [requireDocsAdmin],
});
```
