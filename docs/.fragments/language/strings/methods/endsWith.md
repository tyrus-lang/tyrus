```tyrus
const filename: string = "main.tyr";
const isTyrus: boolean = filename.endsWith(".tyr"); // true
```

::: danger error
```tyrus
const filename: string = "main.tyr";
const isTyrus: boolean = filename.endsWith(42); // [!code error]
```

```text
error[E0308]: type mismatch for `endsWith`: expected `string`, found `integer literal`
at /src/main.tyr:2:26
const isTyrus: boolean = filename.endsWith(42);
                         ^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
