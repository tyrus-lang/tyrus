```tyrus
import { parseSocketAddress } from "tyrus:net";

const address = parseSocketAddress("127.0.0.1:99999");
print(address ?? "missing"); // "missing"
```
