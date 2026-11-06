```tyrus
let inventory: Record<string, i32> = { apples: 3, pears: 5 };
inventory["bananas"] = 12;                   // { apples: 3, pears: 5, bananas: 12 }
const missing: i32 = inventory["oranges"] ?? 0; // 0

interface Config {
    host: string;
    port: i32;
    debug: boolean;
}

const defaults: Config = { host: "localhost", port: 8080, debug: false };
const overrides: Partial<Config> = { port: 3000 };
const config: Config = { ...defaults, ...overrides };
// { host: "localhost", port: 3000, debug: false }

const keys: string[] = Object.keys(config);
// ["host", "port", "debug"]
```