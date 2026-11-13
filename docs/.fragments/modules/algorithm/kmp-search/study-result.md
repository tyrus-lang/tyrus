```tyrus
interface KmpSearchInfo {
    text: string;
    pattern: string;
    matches: usize[];
    comparisonCount: usize;
    shiftCount: usize;
    prefixTable: usize[];
    prefixFallbackCount: usize;
}
```

```tyrus
import { KmpSearchInfo, StringSearchInfo, kmpSearchInfo } from "tyrus:algorithm";
const info: KmpSearchInfo = kmpSearchInfo("ababa", "aba");
print(info.text); // ababa
print(info.pattern); // aba
print(info.matches); // [0, 2]
print(info.comparisonCount); // 7
print(info.shiftCount); // 5
print(info.prefixTable); // [0, 0, 1]
print(info.prefixFallbackCount); // 0
```
