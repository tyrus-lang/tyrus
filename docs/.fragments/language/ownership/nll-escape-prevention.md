```tyrus
const invalidEscape = (): &i32 => {
    const x: i32 = 42;
    return &x; // [!code error]
};
```
