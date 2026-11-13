```tyrus
interface Config {
    host?: string;
    port?: i32;
}

type StrictConfig = Required<Config>;
const cfg: StrictConfig = { host: "localhost", port: 8080 };
```

::: danger error
```tyrus
interface Config {
    host?: string;
    port?: i32;
}

type StrictConfig = Required<Config>;
const cfg: StrictConfig = { host: "localhost" }; // [!code error]
```

```text
error[E2741]: property `port` is missing in object literal for type `object`
at /src/main.tyr:7:7
const cfg: StrictConfig = { host: "localhost" };
      ^^^
```
:::
