```tyrus
const filename: string = "main.tyr";
const isSource: boolean = filename.startsWith("main."); // true
```

::: danger error
```tyrus
const filename: string = "main.tyr";
const isSource: boolean = filename.startsWith(true); // [!code error]
```

```text
error[E0308]: type mismatch for `startsWith`: expected `string`, found `boolean`
at /src/main.tyr:2:27
const isSource: boolean = filename.startsWith(true);
                          ^^^^^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
