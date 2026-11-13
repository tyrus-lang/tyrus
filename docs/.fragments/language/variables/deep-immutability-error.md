```tyrus
const numbers: i32[] = [1, 2, 3];

numbers[0] = 99; // [!code error]
```

```text
error[E0384]: cannot assign through immutable `numbers`
at /src/main.tyr:3:1
numbers[0] = 99;
^^^^^^^
help: declare `numbers` with `let` to allow assignment
```
