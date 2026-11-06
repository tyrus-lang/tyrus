```tyrus
interface ZAlgorithmInfo {
    text: string;
    zTable: usize[];
    comparisonCount: usize;
    windowExtensionCount: usize;
}
```

```tyrus
import { ZAlgorithmInfo, zAlgorithmInfo } from "tyrus:algorithm";
const info: ZAlgorithmInfo = zAlgorithmInfo("aabcaab");
print(info.text); // aabcaab
print(info.zTable); // [0, 1, 0, 0, 3, 1, 0]
print(info.comparisonCount); // 9
print(info.windowExtensionCount); // 4
```
