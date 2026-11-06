```tyrus
const colors: string[] = ["red", "green", "blue"];
const updated: string[] = colors.with(1, "yellow");
// ["red", "yellow", "blue"]
// colors is unchanged
```

::: danger error
```tyrus
const colors: string[] = ["red", "green", "blue"];
const updated: string[] = colors.with(1, 42); // [!code error]
```

```text
error[E0308]: type mismatch for `with`: expected `string`, found `integer literal`
at /src/main.tyr:2:27
const updated: string[] = colors.with(1, 42);
                          ^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::