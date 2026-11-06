```tyrus
const left: f64 = 6.0;
const right: f64 = 4.0;
const x: f32 = 3.0;
const y: f32 = 4.0;
const sum: f64 = left.algebraicAdd(right);
const difference: f64 = left.algebraicSub(right);
const product: f32 = x.algebraicMul(y);
const quotient: f64 = left.algebraicDiv(right);
const remainder: f64 = left.algebraicRem(right);

print(sum);        // 10.0
print(difference); // 2.0
print(product);    // 12.0
print(quotient);   // 1.5
print(remainder);  // 2.0
```
