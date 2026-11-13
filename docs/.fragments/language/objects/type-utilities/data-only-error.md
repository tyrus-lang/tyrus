```tyrus
interface ApiPayload {
    id: i32;
    name: string;
    validate: () => boolean;
}

type Data = DataOnly<ApiPayload>;
const bad: Data = { id: 1, name: "Ada", validate: () => true }; // [!code error]
```

```text
error[E2353]: object literal specifies unknown property `validate`
at /src/main.tyr:8:41
const bad: Data = { id: 1, name: "Ada", validate: () => true };
                                        ^^^^^^^^
```
