```tyrus
const repeated: [i32; 5] = [0; 5];
const values: [i32; 5] = [1, 2, 3, 4, 5];

print(repeated); // [0, 0, 0, 0, 0]
print(values); // [1, 2, 3, 4, 5]
```

::: danger error
```tyrus
const most: [u8; 3] = [10, 20, 30, 40]; // [!code error]
```
```text
error[E0308]: fixed-length type requires 3 elements, but this initializer contains 4
at /src/main.tyr:1:23
const most: [u8; 3] = [10, 20, 30, 40];
                      ^^^^^^^^^^^^^^^^
help: provide exactly 3 elements in this initializer
```
:::
