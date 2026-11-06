```tyrus
interface Example {
    @unique()
    permissions: string[];
}

const valid: Example = { permissions: ["read", "write"] };
```

::: danger error
```tyrus
const invalid: Example = { permissions: ["read", "write", "read"] }; // [!code error]
```

```text
error[validator-decorations-failed]: `@unique()` failed for `permissions`: array items must be unique
at /src/main.tyr:6:41
const invalid: Example = { permissions: ["read", "write", "read"] };
                                        ^^^^^^^^^^^^^^^^^^^^^^^^^
```
:::
