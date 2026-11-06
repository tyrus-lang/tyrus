```tyrus
import { heavyLightDecomposition } from "tyrus:algorithm";

const adj: usize[][] = [[1, 2], [0, 3, 4], [0, 5, 6], [1], [1], [2], [2]];
const hld = heavyLightDecomposition(adj, 0);

print(hld.queryPath(3, 4));  // [[3, 3], [1, 2]]
print(hld.queryPath(5, 4));  // [[4, 5], [3, 3], [0, 1]]
print(hld.position(3));  // 2
print(hld.head(3));  // 0
```
