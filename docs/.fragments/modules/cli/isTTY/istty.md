```tyrus
import { isTTY } from "tyrus:cli";

const interactive: boolean = isTTY();
if (interactive) {
    print("stdout is a terminal");
}
```
