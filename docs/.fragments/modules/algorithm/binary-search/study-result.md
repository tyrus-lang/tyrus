```tyrus
interface SearchInfo {
    values: i32[];
    target: i32;
    index: usize?;
    comparisonCount: usize;
    probeCount: usize;
}
```

```tyrus
import { SearchInfo, binarySearchInfo } from "tyrus:algorithm";

const info: SearchInfo = binarySearchInfo([1, 2, 4, 7, 9], 7);
print(info.values); // [1, 2, 4, 7, 9]
print(info.target); // 7
print(info.index ?? 99); // 3
print(info.comparisonCount); // 3
print(info.probeCount); // 3
```
