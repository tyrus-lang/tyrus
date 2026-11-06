```tyrus
// src/http/handler.tyr
import { sign } from "../auth/token"; // [!code error]

export const handle = (id: i32): i32 => sign(id);
```
