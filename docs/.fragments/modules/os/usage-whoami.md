```tyrus
import { whoami } from "tyrus:os";

const realName: string? = whoami();
if (realName) {
    print(`hello, ${realName}`);
}
```
