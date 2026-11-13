```tyrus
const positive: f64 = Infinity;
const negative: f32 = -Infinity;
const invalid: f128 = NaN;

print(isNaN(invalid)); // true
print(NaN === NaN);    // false
print(NaN !== NaN);    // true
```
