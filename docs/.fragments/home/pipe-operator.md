```tyrus
const main = (): void => {
    const double = (n: i64): i64 => n * 2;
    const increment = (n: i64): i64 => n + 1;
    const clamp = (value: i64, min: i64, max: i64): i64 => value < min ? min : value > max ? max : value;

    // Pipe chains read top-to-bottom
    const result: i64 = 5
        |> double
        |> increment
        |> double;  // 5 -> 10 -> 11 -> 22

    // Topic token % places the piped value exactly
    const clamped: i64 = 80 |> clamp(30, %, 100); // 80
    print(clamped); // 80
};
```
