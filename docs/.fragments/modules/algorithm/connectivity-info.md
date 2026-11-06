```tyrus
interface FindBridgesInfo {
    examinedEdgeCount: usize;
    discoveryIndices: usize[];
    lowLinks: usize[];
    resultCount: usize;
    bridges: [usize, usize][];
}
```

```tyrus
import { ConnectivityInfo, FindBridgesInfo, findBridgesInfo } from "tyrus:algorithm";
const info: FindBridgesInfo = findBridgesInfo([[1, 2], [0, 2, 3], [0, 1], [1]]);
print(info.bridges); // [[1, 3]]
print(info.examinedEdgeCount); // 8
print(info.discoveryIndices); // [0, 1, 2, 3]
print(info.lowLinks); // [0, 0, 0, 3]
print(info.resultCount); // 1
```
