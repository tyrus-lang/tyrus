```tyrus
interface Example {
    @length(8)
    code: string;
}

const valid: Example = { code: "12345678" };
```

::: danger error
```tyrus
const invalid: Example = { code: "1234567" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@length(8)` failed for `code`: value must have exactly 8 characters
at /src/main.tyr:6:34
const invalid: Example = { code: "1234567" };
                                 ^^^^^^^^^
```
:::
