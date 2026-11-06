```tyrus
const main = (): void => {
    let x: i32 = 5;
    const r: &mut i32 = &mut x;
    print(r); // last use of `r`
    x += 1;   // OK: `r` is dead, so `x` can be mutated
};
```
