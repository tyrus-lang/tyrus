```tyrus
import { SocketAddress, parseSocketAddress } from "tyrus:net";

const address: SocketAddress? = parseSocketAddress("[2001:db8::10]:443");
if (address) {
    print(address.port); // 443
}
```
