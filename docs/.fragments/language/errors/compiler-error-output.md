```text
error[E0308]: type mismatch for `maxUsers`: expected `u32`, found `string`
at /src/main.tyr:1:7
const maxUsers: u32 = "100";
      ^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
