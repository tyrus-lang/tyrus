```tyrus
const wide: i64 = 9999999999;
const narrowed = wide as i32?;                  // empty, does not fit an i32
const fits = 7 as i32?;                         // 7
const truncated = parseFloat("2.9") as i32?;    // 2
const unbounded = parseFloat("1e30") as i32?;   // empty
```
