```tyrus
const maxRetries: i32 = 3;

maxRetries = 5; // [!code error]
```

```text
error[E0384]: cannot assign to immutable variable `maxRetries`
at /src/main.tyr:3:1
maxRetries = 5;
^^^^^^^^^^
help: declare `maxRetries` with `let` to allow assignment
```
