```tyrus
interface KruskalInfo {
    edges: [usize, usize, i32][];
    totalWeight: i32;
    componentCount: usize;
    examinedEdgeCount: usize;
    acceptedEdgeCount: usize;
    unionAttemptCount: usize;
    successfulUnionCount: usize;
    sortedEdgeExaminationCount: usize;
}
```

```tyrus
import { KruskalInfo, SpanningTreeInfo, kruskalInfo } from "tyrus:algorithm";

const info: KruskalInfo = kruskalInfo([[0, 1, 4], [0, 2, 2], [1, 2, 1], [1, 3, 5], [3, 4, 2]], 5);
print(info.edges); // [[1, 2, 1], [0, 2, 2], [3, 4, 2], [1, 3, 5]]
print(info.totalWeight); // 10
print(info.componentCount); // 1
print(info.examinedEdgeCount); // 5
print(info.acceptedEdgeCount); // 4
print(info.unionAttemptCount); // 5
print(info.successfulUnionCount); // 4
print(info.sortedEdgeExaminationCount); // 5
```
