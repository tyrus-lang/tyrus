```tyrus
import { terminalSize } from "tyrus:os";

const size: [u16, u16]? = terminalSize();
if (size) {
    print(`${size[0]} columns x ${size[1]} rows`);
}
```
