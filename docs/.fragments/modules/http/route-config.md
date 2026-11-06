```tyrus
app.get("/users/:id", {
    params: UserParams,
    response: { 200: UserResponse },
    config: {
        bodyLimit: "5mb",
        rateLimit: { max: 20, timeWindow: "1 minute" },
    },
    handler: getUser,
});
```
