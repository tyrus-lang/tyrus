```text
error[E0308]: type mismatch for `argument 1 of `takesI32``: expected `i32`, found `i64`
at /src/main.tyr:7:1
takesI32(large);
^^^^^^^^^^^^^^^
help: `i64` does not convert to `i32` on its own; write the conversion explicitly with `as i32`
```
