```tyrus
interface Example {
    @integer()
    count: f64;
}

const valid: Example = { count: 42.0 };
```

::: danger error
```tyrus
const invalid: Example = { count: 3.14 }; // [!code error]
```

```text
error[validator-decorations-failed]: `@integer()` failed for `count`: value must be an integer
at /src/main.tyr:6:35
const invalid: Example = { count: 3.14 };
                                  ^^^^
```
:::
