```tyrus
interface ExponentialSearchInfo extends SearchInfo {
    rangeExpansions: usize;
    finalLow: usize;
    finalHigh: usize;
}
```

```tyrus
import { ExponentialSearchInfo, exponentialSearchInfo } from "tyrus:algorithm";

const info: ExponentialSearchInfo = exponentialSearchInfo([1, 2, 4, 7, 9], 9);
print(info.values); // [1, 2, 4, 7, 9]
print(info.index ?? 99); // 4
print(info.comparisonCount); // 4
print(info.probeCount); // 2
print(info.rangeExpansions); // 2
print(info.finalLow); // 2
print(info.finalHigh); // 5
```
