```tyrus
interface HungarianInfo {
    assignments: [usize, usize][];
    totalCost: i32;
    potentialAdjustmentCount: usize;
    matrixIterationCount: usize;
}
```

```tyrus
import { HungarianInfo, hungarianInfo } from "tyrus:algorithm";
const info: HungarianInfo = hungarianInfo([[4, 1], [2, 3]]);
print(info.assignments); // [[1, 0], [0, 1]]
print(info.totalCost); // 3
print(info.potentialAdjustmentCount); // 4
print(info.matrixIterationCount); // 2
```
