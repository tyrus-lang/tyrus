```tyrus
const version: string = "v0.1.0";
const numeric: string = version.substring(1); // "0.1.0"
```

::: danger error
```tyrus
const version: string = "v0.1.0";
const numeric: string = version.substring("1"); // [!code error]
```

```text
error[E0308]: type mismatch for `substring`: expected `i64`, found `string`
at /src/main.tyr:2:25
const numeric: string = version.substring("1");
                        ^^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
