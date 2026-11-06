```tyrus
import { dijkstra } from "tyrus:algorithm";

const result: [usize[], i32]? = dijkstra([[[1, 4], [2, 2]], [[3, 5]], [[3, 1]], []], 0, 3);
if (result) { print(result[0], result[1]); } // [0, 2, 3] 3
```
