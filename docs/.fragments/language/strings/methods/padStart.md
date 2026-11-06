```tyrus
const id: string = "42";
const padded: string = id.padStart(5, "0"); // "00042"
```

::: danger error
```tyrus
const id: string = "42";
const padded: string = id.padStart("5"); // [!code error]
```

```text
error[E0308]: type mismatch for `padStart`: expected `usize`, found `string`
at /src/main.tyr:2:24
const padded: string = id.padStart("5");
                       ^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
