```tyrus
const digits: i32[] = [1, 2, 3, 2, 1];
const last: i64 = digits.lastIndexOf(2);
// 3
```

::: danger error
```tyrus
const digits: i32[] = [1, 2, 3, 2, 1];
const last: i64 = digits.lastIndexOf("2"); // [!code error]
```

```text
error[E0308]: type mismatch for `lastIndexOf`: expected `i32`, found `string`
at /src/main.tyr:2:19
const last: i64 = digits.lastIndexOf("2");
                  ^^^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
