```tyrus
// src/auth/token.tyr
export const sign = (id: i32): i32 => id * 31 + 7;
export const label = (id: i32): string => `token#${id}`;
```

```tyrus
// src/http/handler.tyr
import { label } from "../auth/token";

export const handle = (id: i32): string => label(id);
```
