```tyrus
import { UserInfo, userInfo } from "tyrus:os";

const info: UserInfo = userInfo();
print(`${info.username} (${info.shell})`);
```
