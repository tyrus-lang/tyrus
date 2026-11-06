```tyrus
import { SocketAddress, parseSocketAddress } from "tyrus:net";

const endpoint: SocketAddress? = parseSocketAddress("[2001:db8::10]:443");
if (endpoint) {
    print(endpoint.ip.address, endpoint.ip.family, endpoint.port); // "2001:db8::10", "IPv6", 443
}
```
