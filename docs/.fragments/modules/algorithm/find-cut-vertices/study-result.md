```tyrus
interface FindCutVerticesInfo {
    examinedEdgeCount: usize;
    discoveryIndices: usize[];
    lowLinks: usize[];
    resultCount: usize;
    cutVertices: usize[];
}
```

```tyrus
import { ConnectivityInfo, FindCutVerticesInfo, findCutVerticesInfo } from "tyrus:algorithm";
const info: FindCutVerticesInfo = findCutVerticesInfo([[1, 2], [0, 2, 3], [0, 1], [1]]);
print(info.cutVertices); // [1]
print(info.examinedEdgeCount); // 8
print(info.discoveryIndices); // [0, 1, 2, 3]
print(info.lowLinks); // [0, 0, 0, 3]
print(info.resultCount); // 1
```
