```tyrus
import { loadAvg } from "tyrus:os";

const load: f64[] = loadAvg();
print(`1m ${load[0]}, 5m ${load[1]}, 15m ${load[2]}`);
```
