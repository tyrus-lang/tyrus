```tyrus
import { centroidDecomposition } from "tyrus:algorithm";

const single: usize[][] = [[]];
const ct = centroidDecomposition(single);

print(ct.parentOf(0) ?? 0);  // 0 (no parent)
print(ct.depth(0));  // 0
```
