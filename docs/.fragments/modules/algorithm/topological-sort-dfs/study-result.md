```tyrus
interface TopologicalSortDfsInfo {
    order: usize[];
    processedVertexCount: usize;
    examinedEdgeCount: usize;
    cycleDetected: bool;
    finalStates: usize[];
    maxDepth: usize;
    backEdge: [usize, usize]?;
}
```

```tyrus
import { TopologicalSortDfsInfo, TopologicalSortInfo, topologicalSortDfsInfo } from "tyrus:algorithm";
const info: TopologicalSortDfsInfo = topologicalSortDfsInfo([[1, 2], [3], [3], []]);
print(info.order); // [0, 2, 1, 3]
print(info.processedVertexCount); // 4
print(info.examinedEdgeCount); // 4
print(info.cycleDetected); // false
print(info.finalStates); // [2, 2, 2, 2]
print(info.maxDepth); // 3
print(info.backEdge ?? [9usize, 9usize]); // [9, 9]
```
