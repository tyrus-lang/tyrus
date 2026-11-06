```tyrus
import { parseIpAddress } from "tyrus:net";

const address = parseIpAddress("not-an-ip");
print(address ?? "missing"); // "missing"
```
