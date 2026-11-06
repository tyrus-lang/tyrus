```tyrus
let x: i32 = 5;
if (cond) {
    const r: &i32 = &x;
    print(r); // `r` dies here
}
x += 1; // OK: no active borrow in outer scope
```
