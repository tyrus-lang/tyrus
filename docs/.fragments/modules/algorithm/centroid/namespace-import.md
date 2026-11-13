```tyrus
import * as alg from "tyrus:algorithm";

const ct = alg.centroidDecomposition([[1], [0, 2], [1]]);
print(ct.parentOf(1) ?? 0);  // 0 (no parent)
print(ct.depth(0));  // 1
```
