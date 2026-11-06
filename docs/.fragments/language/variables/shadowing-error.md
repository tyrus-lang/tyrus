```tyrus
let count: i32 = 1;

let count: i32 = 2; // [!code error]
```

```text
error[E0428]: identifier `count` is declared more than once in this scope
at /src/main.tyr:3:5
let count: i32 = 2;
    ^^^^^
related: previous declaration of `count` is here
at /src/main.tyr:1:5
let count: i32 = 1;
    ^^^^^
```
