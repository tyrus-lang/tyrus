```tyrus
interface Example {
    @nonNegative()
    balance: f64;
}

const valid: Example = { balance: 0.0 };
```

::: danger error
```tyrus
const invalid: Example = { balance: -1.0 }; // [!code error]
```

```text
error[validator-decorations-failed]: `@nonNegative()` failed for `balance`: value must be non-negative
at /src/main.tyr:6:37
const invalid: Example = { balance: -1.0 };
                                    ^^^^
```
:::
