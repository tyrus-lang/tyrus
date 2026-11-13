```tyrus
import { BatteryInfo, battery } from "tyrus:os";

const info: BatteryInfo? = battery();
if (info) {
    print(`${info.level * 100.0}%, charging: ${info.charging}`);
}
```
