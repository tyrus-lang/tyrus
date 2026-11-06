```tyrus
interface TopologicalSortKahnInfo {
    order: usize[];
    processedVertexCount: usize;
    examinedEdgeCount: usize;
    cycleDetected: bool;
    initialIndegrees: usize[];
    indegreeDecrementCount: usize;
    maxQueueSize: usize;
}
```

```tyrus
import { TopologicalSortInfo, TopologicalSortKahnInfo, topologicalSortKahnInfo } from "tyrus:algorithm";
const info: TopologicalSortKahnInfo = topologicalSortKahnInfo([[1, 2], [3], [3], []]);
print(info.order); // [0, 1, 2, 3]
print(info.processedVertexCount); // 4
print(info.examinedEdgeCount); // 4
print(info.cycleDetected); // false
print(info.initialIndegrees); // [0, 1, 1, 2]
print(info.indegreeDecrementCount); // 4
print(info.maxQueueSize); // 2
```
