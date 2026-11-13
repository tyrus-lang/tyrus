```tyrus
const consume = (data: string[]): void => {};

const s: string[] = ["tyrus"];
const r: &string[] = &s;
print(r[0]); // last use of `r`
consume(s);  // OK: borrow ended, `s` can be moved
```
