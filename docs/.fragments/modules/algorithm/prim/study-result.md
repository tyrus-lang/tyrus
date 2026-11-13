```tyrus
interface PrimInfo {
    edges: [usize, usize, i32][];
    totalWeight: i32;
    componentCount: usize;
    examinedEdgeCount: usize;
    acceptedEdgeCount: usize;
    heapPushCount: usize;
    heapPopCount: usize;
    keyUpdateCount: usize;
}
```

```tyrus
import { PrimInfo, SpanningTreeInfo, primInfo } from "tyrus:algorithm";

const info: PrimInfo = primInfo([[[1, 4], [2, 2]], [[0, 4], [2, 1], [3, 5]], [[0, 2], [1, 1], [3, 8], [4, 10]], [[1, 5], [2, 8], [4, 2]], [[2, 10], [3, 2]]], 0);
print(info.edges); // [[0, 2, 2], [2, 1, 1], [1, 3, 5], [3, 4, 2]]
print(info.totalWeight); // 10
print(info.componentCount); // 1
print(info.examinedEdgeCount); // 12
print(info.acceptedEdgeCount); // 4
print(info.heapPushCount); // 7
print(info.heapPopCount); // 7
print(info.keyUpdateCount); // 7
```
