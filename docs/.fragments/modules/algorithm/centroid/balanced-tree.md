```tyrus
import { centroidDecomposition } from "tyrus:algorithm";

const adj: usize[][] = [[1, 2], [0, 3, 4], [0, 5, 6], [1], [1], [2], [2]];
const ct = centroidDecomposition(adj);

print(ct.parentOf(0) ?? 0);  // 0 (no parent, global centroid)
print(ct.depth(0));  // 0
print(ct.parentOf(3) ?? 0);  // 1
print(ct.depth(3));  // 2
print(ct.parentOf(5) ?? 0);  // 2
print(ct.depth(5));  // 2
```
