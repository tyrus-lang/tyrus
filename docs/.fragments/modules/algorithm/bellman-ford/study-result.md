```tyrus
interface BellmanFordInfo {
    distances: i32[];
    predecessors: usize?[];
    relaxationCount: usize;
    processedVertexCount: usize;
    examinedEdgeCount: usize;
    passCount: usize;
    negativeCycleDetected: boolean;
    negativeCycleEdge: [usize, usize, i32]?;
}
```

```tyrus
import { BellmanFordInfo, ShortestPathInfo, bellmanFordInfo } from "tyrus:algorithm";

const info: BellmanFordInfo = bellmanFordInfo([[[1, 4], [2, 1]], [[3, 1]], [[1, 2], [3, 5]], []], 0);
print(info.distances); // [0, 3, 1, 4]
print(info.predecessors[3] ?? 99); // 1
print(info.relaxationCount); // 5
print(info.processedVertexCount); // 4
print(info.examinedEdgeCount); // 20
print(info.passCount); // 3
print(info.negativeCycleDetected); // false
const cycleEdge = info.negativeCycleEdge;
print(cycleEdge ? "detected" : "none"); // none
```
