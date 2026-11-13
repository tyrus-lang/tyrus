```tyrus
import * as alg from "tyrus:algorithm";

const lca = alg.lcaBinaryLifting([[1], [0, 2], [1]], 0);
print(lca.query(1, 2));  // 1
print(lca.query(0, 2));  // 0
```
