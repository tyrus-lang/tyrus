```tyrus
const first: i32[] = [1, 2, 3];
const second: i32[] = [4, 5, 6];
const merged: i32[] = [...first, ...second];

interface Config {
    host: string;
    port: i32;
}

const base: Config = { host: "localhost", port: 8080 };
const overrides: Partial<Config> = { port: 3000 };
const merged_config: Config = { ...base, ...overrides };
```
