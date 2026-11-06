```tyrus
const makeAdder = (base: i32): (value: i32) => i32 => {
    return (value: i32): i32 => base + value;
};

const add5 = makeAdder(5); // inferred as `(i32) => i32`
print(add5(3)); // 8
```
