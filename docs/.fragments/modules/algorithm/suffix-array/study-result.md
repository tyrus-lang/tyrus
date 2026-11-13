```tyrus
interface SuffixArrayInfo {
    text: string;
    suffixArray: usize[];
    sortRoundCount: usize;
    comparisonCount: usize;
    comparedByteCount: usize;
}
```

```tyrus
import { SuffixArrayInfo, suffixArrayInfo } from "tyrus:algorithm";

const info: SuffixArrayInfo = suffixArrayInfo("banana");
print(info.text); // banana
print(info.suffixArray); // [5, 3, 1, 0, 4, 2]
print(info.sortRoundCount); // 3
print(info.comparisonCount); // 11
print(info.comparedByteCount); // 14
```
