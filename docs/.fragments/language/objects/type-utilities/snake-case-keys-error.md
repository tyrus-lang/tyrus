```tyrus
interface ApiConfig {
    dbHost: string;
    dbPort: i32;
}

type Legacy = SnakeCaseKeys<ApiConfig>;
const bad: Legacy = { dbHost: "localhost", dbPort: 5432 }; // [!code error]
```

```text
error[E2353]: object literal specifies unknown property `dbHost`
at /src/main.tyr:7:23
const bad: Legacy = { dbHost: "localhost", dbPort: 5432 };
                      ^^^^^^
```
