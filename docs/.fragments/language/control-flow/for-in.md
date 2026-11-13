```tyrus
interface Config {
    host: string;
    port: i32;
}

const config: Config = { host: "localhost", port: 8080 };

for (const key in config) {
    print(key); // "host", "port"
}
```
