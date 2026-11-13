```tyrus
// src/auth/token.tyr
const sign = (id: i32): i32 => id * 31 + 7;
const label = (id: i32): string => `token#${id}`;

export protect { sign };
export { label };
```
