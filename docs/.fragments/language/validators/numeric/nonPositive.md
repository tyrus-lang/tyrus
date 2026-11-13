```tyrus
interface Example {
    @nonPositive()
    withdrawal: f64;
}

const valid: Example = { withdrawal: 0.0 };
```

::: danger error
```tyrus
const invalid: Example = { withdrawal: 5.0 }; // [!code error]
```

```text
error[validator-decorations-failed]: `@nonPositive()` failed for `withdrawal`: value must be non-positive
at /src/main.tyr:6:40
const invalid: Example = { withdrawal: 5.0 };
                                       ^^^
```
:::
