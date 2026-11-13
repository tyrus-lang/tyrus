```tyrus
interface TraversalInfo {
    start: usize;
    visitOrder: usize[];
    visitedCount: usize;
    examinedEdgeCount: usize;
    maxFrontierSize: usize;
    maxDepth: usize;
}
```

```tyrus
import { TraversalInfo, bfsInfo } from "tyrus:algorithm";

const info: TraversalInfo = bfsInfo([[1, 2], [3], [3], []], 0);
print(info.visitOrder); // [0, 1, 2, 3]
print(info.visitedCount); // 4
print(info.examinedEdgeCount); // 4
print(info.maxFrontierSize); // 2
print(info.maxDepth); // 2
```
