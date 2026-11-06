```tyrus
interface DinicInfo { maxFlow: i32; augmentationCount: usize; examinedEdgeCount: usize; flowEdges: [usize, usize, i32][]; levelGraphCount: usize; blockingFlowCount: usize; currentArcAdvanceCount: usize; }
```

```tyrus
import { DinicInfo, FlowInfo, dinicInfo } from "tyrus:algorithm";
const info: DinicInfo = dinicInfo([[0, 3, 2, 0], [0, 0, 1, 2], [0, 0, 0, 3], [0, 0, 0, 0]], 0, 3);
print(info.maxFlow); // 5
print(info.augmentationCount); // 5
print(info.examinedEdgeCount); // 16
print(info.flowEdges); // [[0, 1, 3], [0, 2, 2], [1, 2, 1], [1, 3, 2], [2, 3, 3]]
print(info.levelGraphCount); // 6
print(info.blockingFlowCount); // 5
print(info.currentArcAdvanceCount); // 16
```
