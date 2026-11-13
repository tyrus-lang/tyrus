```tyrus
let counter: i32 = 0;

counter = "two"; // [!code error]
```

```text
error[E0308]: type mismatch for `counter`: expected `i32`, found `string`
at /src/main.tyr:3:11
counter = "two";
          ^^^^^
help: change the value or the annotation so their types match
```
