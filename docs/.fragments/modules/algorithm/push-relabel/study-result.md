```tyrus
interface PushRelabelInfo { maxFlow: i32; augmentationCount: usize; examinedEdgeCount: usize; flowEdges: [usize, usize, i32][]; pushCount: usize; relabelCount: usize; dischargeCount: usize; maxHeight: usize; }
```

```tyrus
import { FlowInfo, PushRelabelInfo, pushRelabelInfo } from "tyrus:algorithm";
const info: PushRelabelInfo = pushRelabelInfo([[0, 3, 2, 0], [0, 0, 1, 2], [0, 0, 0, 3], [0, 0, 0, 0]], 0, 3);
print(info.maxFlow); // 5
print(info.augmentationCount); // 5
print(info.examinedEdgeCount); // 16
print(info.flowEdges); // [[0, 1, 3], [0, 2, 2], [1, 2, 1], [1, 3, 2], [2, 3, 3]]
print(info.pushCount); // 5
print(info.relabelCount); // 4
print(info.dischargeCount); // 5
print(info.maxHeight); // 4
```
