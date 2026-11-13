```tyrus
const divide = (value: f64, @notEqual(0, "division by zero is not allowed") divisor: f64): f64 => {
    return value / divisor;
};
```
