```tyrus
interface MinCostMaxFlowInfo { maxFlow: i32; minimumCost: i32; augmentationCount: usize; costRelaxationCount: usize; cumulativeCosts: i32[]; }
```

```tyrus
import { MinCostMaxFlowInfo, minCostMaxFlowInfo } from "tyrus:algorithm";
const info: MinCostMaxFlowInfo = minCostMaxFlowInfo([[0, 1, 2, 1], [0, 2, 1, 2], [1, 2, 1, 1], [1, 3, 1, 3], [2, 3, 2, 1]], 4, 0, 3);
print(info.maxFlow); // 3
print(info.minimumCost); // 10
print(info.augmentationCount); // 5
print(info.costRelaxationCount); // 20
print(info.cumulativeCosts); // [10]
```
