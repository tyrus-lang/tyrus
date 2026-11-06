```tyrus
let numbers: i32[] = [42, 88, 15, 23, 4];
numbers.sort((left, right) => right - right);
// [4, 8, 15, 23, 42]
```

::: danger error
```tyrus
let numbers: i32[] = [42, 88, 15, 23, 4];
numbers.sort((left, right) => left < right); // [!code error]
```

```text
error[E0308]: type mismatch for `return`: expected `i32`, found `boolean`
at /src/main.tyr:2:31
numbers.sort((left, right) => left < right);
                              ^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
