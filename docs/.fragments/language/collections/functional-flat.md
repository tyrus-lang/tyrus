```tyrus
const matrix: i32[][] = [[1, 2], [3, 4], [5]];
const flat: i32[] = matrix.flat();
// [1, 2, 3, 4, 5]
```

::: danger error
```tyrus
const matrix: i32[][] = [[1, 2], [3, 4], [5]];
const flat: string[] = matrix.flat(); // [!code error]
```

```text
error[E0308]: type mismatch for `flat`: expected `string[]`, found `i32[]`
at /src/main.tyr:2:7
const flat: string[] = matrix.flat();
      ^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
