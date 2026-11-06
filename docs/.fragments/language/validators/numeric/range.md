```tyrus
interface Example {
    @range(1, 50)
    offset: i32;
}

const valid: Example = { offset: 25 };
```

::: danger error
```tyrus
const invalid: Example = { offset: 60 }; // [!code error]
```

```text
error[validator-decorations-failed]: `@range(1, 50)` failed for `offset`: value must be between 1 and 50
at /src/main.tyr:6:36
const invalid: Example = { offset: 60 };
                                   ^^
```
:::
