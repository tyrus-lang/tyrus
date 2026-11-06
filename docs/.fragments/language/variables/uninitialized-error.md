```tyrus
let total: i32;

print(total); // [!code error]
```

```text
error[E0381]: `total` is used before being assigned a value
at /src/main.tyr:3:7
print(total);
      ^^^^^
help: assign a value to `total` before reading it
```
