```tyrus
const factor: i32 = 3;
const offset: i32 = 2;
const transform: (i32) => i32 = (value: i32): i32 => value * factor + offset;
print(transform(10)); // 32
```
