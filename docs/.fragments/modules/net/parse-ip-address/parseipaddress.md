```tyrus
import { IpAddress, parseIpAddress } from "tyrus:net";

const address: IpAddress? = parseIpAddress("192.0.2.10");
if (address) {
    print(address.family); // "IPv4"
}
```
