```tyrus
interface Example {
    @negative()
    debt: f64;
}

const valid: Example = { debt: -10.0 };
```

::: danger error
```tyrus
const invalid: Example = { debt: 10.0 }; // [!code error]
```

```text
error[validator-decorations-failed]: `@negative()` failed for `debt`: value must be negative
at /src/main.tyr:6:34
const invalid: Example = { debt: 10.0 };
                                 ^^^^
```
:::
