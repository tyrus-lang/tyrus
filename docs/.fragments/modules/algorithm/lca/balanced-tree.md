```tyrus
import { lcaBinaryLifting } from "tyrus:algorithm";

const adj: usize[][] = [[1, 2], [0, 3, 4], [0, 5, 6], [1], [1], [2], [2]];
const lca = lcaBinaryLifting(adj, 0);

print(lca.query(3, 4));  // 1
print(lca.query(5, 6));  // 2
print(lca.query(3, 6));  // 0
```
