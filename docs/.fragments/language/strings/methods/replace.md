```tyrus
const path: string = "src/main.ts";
const updated: string = path.replace(".ts", ".tyr"); // "src/main.tyr"
```

::: danger error
```tyrus
const path: string = "src/main.ts";
const updated: string = path.replace(".ts", 42); // [!code error]
```

```text
error[E0308]: type mismatch for `replace`: expected `string`, found `integer literal`
at /src/main.tyr:2:25
const updated: string = path.replace(".ts", 42);
                        ^^^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
