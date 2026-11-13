```tyrus
interface Example {
    @max(100)
    limit: i32;
}

const valid: Example = { limit: 50 };
```

::: danger error
```tyrus
const invalid: Example = { limit: 200 }; // [!code error]
```

```text
error[validator-decorations-failed]: `@max(100)` failed for `limit`: value must be less than or equal to 100
at /src/main.tyr:6:35
const invalid: Example = { limit: 200 };
                                  ^^^
```
:::
