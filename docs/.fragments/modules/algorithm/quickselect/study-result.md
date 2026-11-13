```tyrus
interface SelectionInfo {
    values: i32[];
    selected: i32?;
    rank: usize;
    comparisonCount: usize;
    swapCount: usize;
    partitionCount: usize;
}
```

```tyrus
import { SelectionInfo, quickselectInfo } from "tyrus:algorithm";

const info: SelectionInfo = quickselectInfo([7, 1, 4, 9, 2], 2);
print(info.values); // [7, 1, 4, 9, 2]
print(info.selected ?? -1); // 4
print(info.rank); // 2
print(info.comparisonCount); // 10
print(info.swapCount); // 3
print(info.partitionCount); // 1
```
