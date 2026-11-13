```tyrus
interface Example {
    @min(1)
    pageSize: u32;
}

const valid: Example = { pageSize: 5 };
```

::: danger error
```tyrus
const invalid: Example = { pageSize: 0 }; // [!code error]
```

```text
error[validator-decorations-failed]: `@min(1)` failed for `pageSize`: value must be greater than or equal to 1
at /src/main.tyr:6:38
const invalid: Example = { pageSize: 0 };
                                     ^
```
:::
