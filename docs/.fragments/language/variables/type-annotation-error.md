```tyrus
const timeout: u32 = "5000"; // [!code error]
```

```text
error[E0308]: type mismatch for `timeout`: expected `u32`, found `string`
at /src/main.tyr:1:7
const timeout: u32 = "5000";
      ^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
