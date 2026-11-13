```tyrus
import { diskUsage } from "tyrus:fs";

const usage = diskUsage(".");
print(usage.total); // e.g. 500107862016
print(usage.used); // e.g. 250053931008
print(usage.free); // e.g. 249850901760
```
