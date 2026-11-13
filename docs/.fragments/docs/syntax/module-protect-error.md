```tyrus
// src/http/handler.tyr
import { verify } from "../auth/token"; // [!code error]

export const handle = (id: i32): i32 => verify(id);
```
