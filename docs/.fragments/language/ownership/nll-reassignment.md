```tyrus
let a: i32 = 1;
let b: i32 = 2;
let r: &mut i32 = &mut a;
print(r);
r = &mut b; // borrow on `a` ends; borrow on `b` begins
a += 1;     // OK: `a` is no longer borrowed
print(r);
```
