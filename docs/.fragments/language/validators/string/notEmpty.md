```tyrus
interface Example {
    @notEmpty()
    firstName: string;
}

const valid: Example = { firstName: "John" };
```

::: danger error
```tyrus
const invalid: Example = { firstName: "" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@notEmpty()` failed for `firstName`: value must not be empty
at /src/main.tyr:6:39
const invalid: Example = { firstName: "" };
                                      ^^
```
:::
