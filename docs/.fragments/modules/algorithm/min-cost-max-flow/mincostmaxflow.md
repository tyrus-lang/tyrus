```tyrus
import { minCostMaxFlow } from "tyrus:algorithm";

const result: [i32, i32] = minCostMaxFlow([[0, 1, 2, 3]], 2, 0, 1);
print(result[0], result[1]); // 2 6
```
