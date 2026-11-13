```tyrus
import * as alg from "tyrus:algorithm";

const hld = alg.heavyLightDecomposition([[1], [0, 2], [1]], 0);
print(hld.queryPath(1, 2));  // [[1, 2]]
```
