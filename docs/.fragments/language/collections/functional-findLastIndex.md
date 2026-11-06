```tyrus
const duplicates: i32[] = [1, 2, 3, 4, 1];
const lastIdx: i64 = duplicates.findLastIndex(n => n === 2);
// 3
```

::: danger error
```tyrus
const duplicates: i32[] = [1, 2, 3, 4, 1];
const lastIdx: i64 = duplicates.findLastIndex(n => n); // [!code error]
```

```text
error[E0308]: type mismatch for `return`: expected `boolean`, found `i32`
at /src/main.tyr:2:52
const lastIdx: i64 = duplicates.findLastIndex(n => n);
                                                   ^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
