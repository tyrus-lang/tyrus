```tyrus
import { ping, supportsIcmp } from "tyrus:net";

async function probe(): Promise<void> {
    if (!supportsIcmp()) {
        print("ERR_NET_UNSUPPORTED"); // "ERR_NET_UNSUPPORTED"
        return;
    }
    await ping("127.0.0.1");
}
```
