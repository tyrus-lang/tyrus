```tyrus
interface Config {
    db_host: string;
    db_port: i32;
}

type Cfg = CamelCaseKeys<Config>;
const bad: Cfg = { db_host: "localhost", db_port: 5432 }; // [!code error]
```

```text
error[E2353]: object literal specifies unknown property `db_host`
at /src/main.tyr:7:20
const bad: Cfg = { db_host: "localhost", db_port: 5432 };
                   ^^^^^^^
```
