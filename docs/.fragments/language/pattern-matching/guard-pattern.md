```tyrus
const value: i32 = 15;

match (value) {
    (v) if (v < 0) => {
        print("Negative");
    }
    0 => {
        print("Zero");
    }
    (v) if (v % 2 === 0) => {
        print("Positive even");
    }
    _ => {
        print("Positive odd");
    }
}
// Positive odd
```
