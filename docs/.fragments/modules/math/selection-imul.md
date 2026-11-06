```tyrus
const a: i32 = 2000000000;
const b: i32 = 2;
const result: i32 = imul(a, b);
print(result); // -294967296 (C-style 32-bit overflow)
```
