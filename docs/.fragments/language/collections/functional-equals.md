```tyrus
const left: i32[] = [1, 2, 3];
const right: i32[] = [1, 2, 3];
const same: boolean = left.equals(right);
// true

const reordered: boolean = left.equals([3, 2, 1]);
// false
```

::: danger error
```tyrus
const grid: i32[][] = [[1], [2]];
const same: boolean = grid.equals([[1], [2]]); // [!code error]
```

```text
error[E0277]: array method `equals` compares arrays of numbers, `boolean`, `char` or `string`, but the elements are `i32[]`
at /src/main.tyr:2:23
const same: boolean = grid.equals([[1], [2]]);
                      ^^^^^^^^^^^^^^^^^^^^^^^
```
:::
