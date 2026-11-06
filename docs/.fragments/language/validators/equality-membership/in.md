```tyrus
interface Example {
    @in(["red", "green", "blue"])
    color: string;
}

const valid: Example = { color: "blue" };
```

::: danger error
```tyrus
const invalid: Example = { color: "yellow" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@in(["red", "green", "blue"])` failed for `color`: value must be one of ["red", "green", "blue"]
at /src/main.tyr:6:35
const invalid: Example = { color: "yellow" };
                                  ^^^^^^^^
```
:::
