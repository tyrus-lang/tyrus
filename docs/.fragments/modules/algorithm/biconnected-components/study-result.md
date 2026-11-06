```tyrus
interface BiconnectedComponentsInfo {
    examinedEdgeCount: usize;
    discoveryIndices: usize[];
    lowLinks: usize[];
    resultCount: usize;
    components: [usize, usize][][];
}
```

```tyrus
import { BiconnectedComponentsInfo, ConnectivityInfo, biconnectedComponentsInfo } from "tyrus:algorithm";
const info: BiconnectedComponentsInfo = biconnectedComponentsInfo([[1, 2], [0, 2, 3], [0, 1], [1]]);
print(info.components); // [[ [1, 3] ], [ [2, 0], [1, 2], [0, 1] ]]
print(info.examinedEdgeCount); // 8
print(info.discoveryIndices); // [0, 1, 2, 3]
print(info.lowLinks); // [0, 0, 0, 3]
print(info.resultCount); // 2
```
