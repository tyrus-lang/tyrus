```tyrus
interface AStarInfo {
    distances: i32[];
    predecessors: usize?[];
    relaxationCount: usize;
    processedVertexCount: usize;
    examinedEdgeCount: usize;
    heuristicEvaluationCount: usize;
    openedVertexCount: usize;
    closedVertexCount: usize;
    finalCost: i32?;
}
```

```tyrus
import { AStarInfo, ShortestPathInfo, aStarInfo } from "tyrus:algorithm";

const info: AStarInfo = aStarInfo([[[1, 4], [2, 1]], [[3, 1]], [[1, 2], [3, 5]], []], 0, 3, [3, 1, 2, 0]);
print(info.distances); // [0, 3, 1, 4]
print(info.predecessors[3] ?? 99); // 1
print(info.relaxationCount); // 5
print(info.processedVertexCount); // 4
print(info.examinedEdgeCount); // 5
print(info.heuristicEvaluationCount); // 6
print(info.openedVertexCount); // 4
print(info.closedVertexCount); // 4
print(info.finalCost ?? -1); // 4
```
