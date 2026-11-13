```tyrus
const scores: i32[] = [85, 42, 91, 38, 76, 55];
const passing: i32[] = scores.filter(score => score >= 60);
// [85, 91, 76]
```

::: danger error
```tyrus
const scores: i32[] = [85, 42, 91, 38, 76, 55];
const result: i32[] = scores.filter(score => score * 2); // [!code error]
```

```text
error[E0308]: type mismatch for `return`: expected `boolean`, found `i32`
at /src/main.tyr:2:46
const result: i32[] = scores.filter(score => score * 2);
                                             ^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
