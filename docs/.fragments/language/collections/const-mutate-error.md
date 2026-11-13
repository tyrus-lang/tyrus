```tyrus
const fixed: i32[] = [1, 2, 3];

fixed.push(4); // [!code error]
```

```text
error[E0384]: cannot call mutating method `push` on immutable array `fixed`
at /src/main.tyr:3:1
fixed.push(4);
^^^^^
help: declare the binding with `let` to allow assignment
```
