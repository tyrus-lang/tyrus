```tyrus
// src/auth/index.tyr
export * from "./token";
```

```tyrus
// src/auth/login.tyr
import { sign } from "./index";

export const login = (id: i32): i32 => sign(id);
```
