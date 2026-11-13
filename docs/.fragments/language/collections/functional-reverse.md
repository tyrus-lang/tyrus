```tyrus
let stack: i32[] = [1, 2, 3];
stack.reverse();
// [3, 2, 1]
```

::: danger error
```tyrus
const stack: i32[] = [1, 2, 3];
stack.reverse(); // [!code error]
```

```text
error[E0384]: cannot call mutating method `reverse` on immutable array `stack`
at /src/main.tyr:2:1
stack.reverse();
^^^^^
help: declare the binding with `let` to allow assignment
```
:::
