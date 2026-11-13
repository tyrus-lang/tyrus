```tyrus
import { quickselect } from "tyrus:algorithm";

const value: i32? = quickselect([7, 1, 4, 9, 2], 2);
print(value ?? -1); // 4
```
