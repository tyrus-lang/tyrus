```tyrus
interface Example {
    @uuid()
    sessionId: string;
}

const valid: Example = { sessionId: "550e8400-e29b-41d4-a716-446655440000" };
```

::: danger error
```tyrus
const invalid: Example = { sessionId: "not-a-uuid" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@uuid()` failed for `sessionId`: value must be a valid UUID
at /src/main.tyr:6:39
const invalid: Example = { sessionId: "not-a-uuid" };
                                      ^^^^^^^^^^^^
```
:::
