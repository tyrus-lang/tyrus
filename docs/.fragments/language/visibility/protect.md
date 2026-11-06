```tyrus
// src/auth/token.tyr
export protect const sign = (id: i32): i32 => id * 31 + 7;
export const label = (id: i32): string => `token#${id}`;
```

```tyrus
// src/auth/login.tyr
import { sign } from "./token";

export const login = (id: i32): i32 => sign(id);
```

```tyrus
// src/auth/session/refresh.tyr
import { sign } from "../token";

export const refresh = (id: i32): i32 => sign(id) + 1;
```
