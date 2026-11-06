```tyrus
const double = (value: i32): i32 => value * 2;
const apply = (f: (i32) => i32, v: i32): i32 => f(v);

print(apply(double, 21)); // passed as an argument

const numbers: i32[] = [1, 2, 3];
const doubled: i32[] = numbers.map(double); // used as a callback
```
