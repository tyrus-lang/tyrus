```tyrus
interface DijkstraInfo {
    distances: i32[];
    predecessors: usize?[];
    relaxationCount: usize;
    processedVertexCount: usize;
    examinedEdgeCount: usize;
    heapExtractionCount: usize;
    acceptedRelaxationCount: usize;
    finalizedVertexCount: usize;
}
```

```tyrus
import { DijkstraInfo, ShortestPathInfo, dijkstraInfo } from "tyrus:algorithm";

const info: DijkstraInfo = dijkstraInfo([[[1, 4], [2, 1]], [[3, 1]], [[1, 2], [3, 5]], []], 0, 3);
print(info.distances); // [0, 3, 1, 4]
print(info.predecessors[3] ?? 99); // 1
print(info.relaxationCount); // 5
print(info.processedVertexCount); // 4
print(info.examinedEdgeCount); // 5
print(info.heapExtractionCount); // 5
print(info.acceptedRelaxationCount); // 5
print(info.finalizedVertexCount); // 4
```
