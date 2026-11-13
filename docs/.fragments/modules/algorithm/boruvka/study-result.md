```tyrus
interface BoruvkaInfo {
    edges: [usize, usize, i32][];
    totalWeight: i32;
    componentCount: usize;
    examinedEdgeCount: usize;
    acceptedEdgeCount: usize;
    phaseCount: usize;
    componentCountsAfterPhase: usize[];
}
```

```tyrus
import { BoruvkaInfo, SpanningTreeInfo, boruvkaInfo } from "tyrus:algorithm";

const info: BoruvkaInfo = boruvkaInfo([[0, 1, 4], [0, 2, 2], [1, 2, 1], [1, 3, 5], [2, 3, 8], [2, 4, 10], [3, 4, 2]], 5);
print(info.edges); // [[0, 2, 2], [1, 2, 1], [3, 4, 2], [1, 3, 5]]
print(info.totalWeight); // 10
print(info.componentCount); // 1
print(info.examinedEdgeCount); // 14
print(info.acceptedEdgeCount); // 4
print(info.phaseCount); // 2
print(info.componentCountsAfterPhase); // [2, 1]
```
