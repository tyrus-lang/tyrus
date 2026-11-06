```tyrus
interface Example {
    @multipleOf(5)
    quantity: i32;
}

const valid: Example = { quantity: 10 };
```

::: danger error
```tyrus
const invalid: Example = { quantity: 7 }; // [!code error]
```

```text
error[validator-decorations-failed]: `@multipleOf(5)` failed for `quantity`: value must be a multiple of 5
at /src/main.tyr:6:38
const invalid: Example = { quantity: 7 };
                                     ^
```
:::
