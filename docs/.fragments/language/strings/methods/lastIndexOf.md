```tyrus
const path: string = "src/main.tyr";
const lastSlash: i64 = path.lastIndexOf("/"); // 3
```

::: danger error
```tyrus
const path: string = "src/main.tyr";
const lastSlash: i64 = path.lastIndexOf(42); // [!code error]
```

```text
error[E0308]: type mismatch for `lastIndexOf`: expected `string`, found `integer literal`
at /src/main.tyr:2:24
const lastSlash: i64 = path.lastIndexOf(42);
                       ^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
