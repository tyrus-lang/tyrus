```tyrus
// src/http/handler.tyr
import { sign } from "../auth/token";

export const handle = (id: i32): i32 => sign(id); // 38 for id 1
```
