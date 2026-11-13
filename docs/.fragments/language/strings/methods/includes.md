```tyrus
const sentence: string = "tyrus is native";
const found: boolean = sentence.includes("native"); // true
```

::: danger error
```tyrus
const sentence: string = "tyrus is native";
const found: boolean = sentence.includes(42); // [!code error]
```

```text
error[E0308]: type mismatch for `includes`: expected `string`, found `integer literal`
at /src/main.tyr:2:24
const found: boolean = sentence.includes(42);
                       ^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
