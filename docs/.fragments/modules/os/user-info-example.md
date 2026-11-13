```tyrus
import { UserInfo, userInfo } from "tyrus:os";

const info: UserInfo = userInfo();
print(info.username);
print(info.homedir);
print(info.shell);
```
