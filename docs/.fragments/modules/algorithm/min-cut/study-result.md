```tyrus
interface MinCutInfo { cutValue: i32; sourcePartition: usize[]; sinkPartition: usize[]; cutEdges: [usize, usize][]; flowEdges: [usize, usize, i32][]; }
```

```tyrus
import { MinCutInfo, minCutInfo } from "tyrus:algorithm";
const info: MinCutInfo = minCutInfo([[0, 3, 2, 0], [0, 0, 1, 2], [0, 0, 0, 3], [0, 0, 0, 0]], 0, 3);
print(info.cutValue); // 5
print(info.sourcePartition); // [0]
print(info.sinkPartition); // [1, 2, 3]
print(info.cutEdges); // [[0, 1], [0, 2]]
print(info.flowEdges); // [[0, 1, 3], [0, 2, 2], [1, 2, 1], [1, 3, 2], [2, 3, 3]]
```
