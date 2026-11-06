```tyrus
const divider: string = "-".repeat(20); // "--------------------"
const pattern: string = "ab".repeat(3); // "ababab"
```

::: danger error
```tyrus
const divider: string = "-".repeat("20"); // [!code error]
```

```text
error[E0308]: type mismatch for `repeat`: expected `usize`, found `string`
at /src/main.tyr:1:25
const divider: string = "-".repeat("20");
                        ^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
