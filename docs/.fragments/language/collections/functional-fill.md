```tyrus
let buffer: i32[] = [0, 0, 0, 0, 0];

buffer.fill(7);
// [7, 7, 7, 7, 7]
```

::: danger error
```tyrus
let buffer: i32[] = [0, 0, 0, 0, 0];
buffer.fill("x"); // [!code error]
```

```text
error[E0308]: type mismatch for `fill`: expected `i32`, found `string`
at /src/main.tyr:2:1
buffer.fill("x");
^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
