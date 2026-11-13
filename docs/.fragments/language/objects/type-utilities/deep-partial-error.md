```tyrus
interface Config {
    db: {
        host: string;
        port: i32;
    };
    active: boolean;
}

type ConfigPatch = DeepPartial<Config>;
const bad: ConfigPatch = { db: { host: 42 } }; // [!code error]
```

```text
error[E0308]: type mismatch for `db`: expected `{ host?: string; port?: i32 }?`, found `object`
at /src/main.tyr:10:28
const bad: ConfigPatch = { db: { host: 42 } };
                           ^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
