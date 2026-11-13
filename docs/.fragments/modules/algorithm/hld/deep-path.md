```tyrus
import { heavyLightDecomposition } from "tyrus:algorithm";

const path: usize[][] = [[1], [0, 2], [1, 3], [2, 4], [3, 5], [4, 6], [5, 7], [6, 8], [7, 9], [8]];
const hld = heavyLightDecomposition(path, 0);

print(hld.queryPath(9, 1));  // [[1, 9]]
print(hld.queryPath(0, 9));  // [[0, 9]]
```
