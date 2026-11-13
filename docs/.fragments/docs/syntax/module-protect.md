```tyrus
// src/auth/token.tyr
export protect const verify = (id: i32): i32 => id + 1;
export const open = (id: i32): i32 => id + 2;
```

```tyrus
// src/auth/login.tyr
import { verify } from "./token";

export const login = (id: i32): i32 => verify(id);
```

```tyrus
// src/auth/session/refresh.tyr
import { verify } from "../token";

export const refresh = (id: i32): i32 => verify(id);
```
