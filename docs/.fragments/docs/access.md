```tyrus
const environment = process.env.TYRUS_ENV;
const port = process.env.PORT;
const { TYRUS_ENV, PORT } = process.env;
print(`[${environment}] listening on ${port}`); // "[development] listening on 3000"
```
