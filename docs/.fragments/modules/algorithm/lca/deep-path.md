```tyrus
import { lcaBinaryLifting } from "tyrus:algorithm";

const path: usize[][] = [[1], [0, 2], [1, 3], [2, 4], [3, 5], [4, 6], [5, 7], [6, 8], [7, 9], [8]];
const lca = lcaBinaryLifting(path, 0);

print(lca.query(9, 1));  // 1
print(lca.query(9, 8));  // 8
print(lca.query(0, 9));  // 0
```
