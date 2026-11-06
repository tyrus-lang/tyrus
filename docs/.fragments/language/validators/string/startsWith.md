```tyrus
interface Example {
    @startsWith("user_")
    userId: string;
}

const valid: Example = { userId: "user_abc" };
```

::: danger error
```tyrus
const invalid: Example = { userId: "abc_123" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@startsWith("user_")` failed for `userId`: value must start with "user_"
at /src/main.tyr:6:36
const invalid: Example = { userId: "abc_123" };
                                   ^^^^^^^^^
```
:::
