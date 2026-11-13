```tyrus
const a: f64 = 1.0;
const b: f64 = 1.0000001;
const result: boolean = approxEqual(a, b);
print(result); // true
```

```tyrus
const a: f64 = 1.0;
const b: f64 = 1.1;
const result: boolean = approxEqual(a, b, 0.05);
print(result); // false
```
