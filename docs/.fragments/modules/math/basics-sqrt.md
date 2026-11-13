```tyrus
const value: f64 = 16.0;
const result: f64 = sqrt(value);
print(result); // 4.0
```

```tyrus
const value: f64 = -4.0;
const result: f64 = sqrt(value); // [!code error]
```

```text
error[E0420]: math domain error
  --> src/main.tyr:2:14
    |
  2 | const result: f64 = sqrt(value);
    |              ^^^^^ square root of negative number
    |
```
