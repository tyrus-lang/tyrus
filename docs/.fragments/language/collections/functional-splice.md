```tyrus
let values: i32[] = [1, 2, 3, 4];
const removed: i32[] = values.splice(1, 2, 7, 8);
print(values[1]);
print(removed[0]);
```

::: danger error
```tyrus
const values: i32[] = [1, 2, 3, 4];
const removed: i32[] = values.splice(1, 2, 7, 8); // [!code error]
```

```text
error[E0384]: cannot call mutating method `splice` on immutable array `values`
at /src/main.tyr:2:24
const removed: i32[] = values.splice(1, 2, 7, 8);
                       ^^^^^^
help: declare the binding with `let` to allow assignment
```
:::
