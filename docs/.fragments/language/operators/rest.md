```tyrus
const log = (level: string, ...messages: string[]): void => {
    for (const msg of messages) {
        print(`[${level}] ${msg}`); // "[info] server started", "[info] listening on port 3000"
    }
};

log("info", "server started", "listening on port 3000");
```
