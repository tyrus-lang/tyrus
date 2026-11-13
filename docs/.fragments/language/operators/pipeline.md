```tyrus
const double = (n: i64): i64 => n * 2;
const increment = (n: i64): i64 => n + 1;
const clamp = (n: i64, max: i64): i64 => (n > max ? max : n);

const value: i64 = 5
    |> double
    |> clamp(%, 50)
    |> increment;
```
