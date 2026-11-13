```tyrus
const label: string = "Name";
const padded: string = label.padEnd(20, "."); // "Name................"
```

::: danger error
```tyrus
const label: string = "Name";
const padded: string = label.padEnd("20"); // [!code error]
```

```text
error[E0308]: type mismatch for `padEnd`: expected `usize`, found `string`
at /src/main.tyr:2:24
const padded: string = label.padEnd("20");
                       ^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
