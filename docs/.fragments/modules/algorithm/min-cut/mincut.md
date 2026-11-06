```tyrus
import { minCut } from "tyrus:algorithm";

const result: [i32, [usize, usize][]] = minCut([[0, 2], [0, 0]], 0, 1);
print(result[0], result[1].length); // 2 1
```
