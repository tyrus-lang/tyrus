```tyrus
import { aStar } from "tyrus:algorithm";

const result: [usize[], i32]? = aStar([[[1, 4], [2, 2]], [[3, 5]], [[3, 1]], []], 0, 3, [3, 1, 1, 0]);
if (result) { print(result[1]); } // 3
```
