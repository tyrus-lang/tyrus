```tyrus
interface Example {
    @positive()
    amount: f64;
}

const valid: Example = { amount: 5.0 };
```

::: danger error
```tyrus
const invalid: Example = { amount: -5.0 }; // [!code error]
```

```text
error[validator-decorations-failed]: `@positive()` failed for `amount`: value must be positive
at /src/main.tyr:6:36
const invalid: Example = { amount: -5.0 };
                                   ^^^^
```
:::
