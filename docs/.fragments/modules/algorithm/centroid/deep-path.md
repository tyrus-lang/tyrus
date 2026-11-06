```tyrus
import { centroidDecomposition } from "tyrus:algorithm";

const path: usize[][] = [[1], [0, 2], [1, 3], [2, 4], [3, 5], [4]];
const ct = centroidDecomposition(path);

print(ct.parentOf(2) ?? 0);  // 0 (no parent, global centroid)
print(ct.depth(2));  // 0
print(ct.parentOf(0) ?? 0);  // 1
print(ct.depth(0));  // 2
```
