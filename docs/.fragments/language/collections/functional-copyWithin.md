```tyrus
let data: i32[] = [1, 2, 3, 4, 5];

data.copyWithin(0, 3);
// [4, 5, 3, 4, 5]
```

::: danger error
```tyrus
let data: i32[] = [1, 2, 3, 4, 5];
data.copyWithin("x", 3); // [!code error]
```

```text
error[E0308]: type mismatch for `copyWithin`: expected `i64`, found `string`
at /src/main.tyr:2:1
data.copyWithin("x", 3);
^^^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
