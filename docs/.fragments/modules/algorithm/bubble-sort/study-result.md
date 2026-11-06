```tyrus
import { BubbleSortInfo, bubbleSortInfo } from "tyrus:algorithm";

const input: i32[] = [4, 1, 3, 2];
const info: BubbleSortInfo = bubbleSortInfo(input);

print(input); // [4, 1, 3, 2]
print(info.originalValues); // [4, 1, 3, 2]
print(info.sortedValues); // [1, 2, 3, 4]
print(info.comparisonCount); // 6
print(info.swapCount); // 4
print(info.passCount); // 3
```
