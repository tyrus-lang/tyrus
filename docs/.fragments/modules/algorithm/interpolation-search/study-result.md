```tyrus
interface InterpolationSearchInfo extends SearchInfo {
    estimatedPositions: usize[];
}
```

```tyrus
import { InterpolationSearchInfo, interpolationSearchInfo } from "tyrus:algorithm";

const info: InterpolationSearchInfo = interpolationSearchInfo([1, 2, 4, 7, 9], 4);
print(info.values); // [1, 2, 4, 7, 9]
print(info.index ?? 99); // 2
print(info.comparisonCount); // 2
print(info.probeCount); // 2
print(info.estimatedPositions); // [1, 2]
```
