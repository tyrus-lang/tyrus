```tyrus
const values: i32[] = [10, 20, 30, 40];
const total: i32 = values.reduce((sum, value) => sum + value, 0);
// 100
```

::: danger error
```tyrus
const values: i32[] = [10, 20, 30, 40];
const total: i64 = values.reduce((sum, value) => sum + value, 0); // [!code error]
```

```text
error[E0308]: type mismatch for `total`: expected `i64`, found `i32`
at /src/main.tyr:2:7
const total: i64 = values.reduce((sum, value) => sum + value, 0);
      ^^^^^
help: `i32` does not convert to `i64` on its own; write the conversion explicitly with `as i64`
```
:::
