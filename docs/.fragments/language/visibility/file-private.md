```tyrus
// src/auth/token.tyr
const salt = (): i32 => 31;

export const sign = (id: i32): i32 => id * salt() + 7;
```
