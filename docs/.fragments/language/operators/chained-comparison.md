```tyrus
const temp: u32 = 25;
print(0 < temp < 100); // true

const age: i32 = 25;
print(18 <= age < 65); // true

const getValue = (): i32 => 25;
print(10 < getValue() <= 50); // true - getValue() runs once, not twice
```
