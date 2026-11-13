```tyrus
interface TarjanSccInfo {
    components: usize[][];
    componentCount: usize;
    examinedEdgeCount: usize;
    stackOperationCount: usize;
    discoveryIndices: usize[];
    lowLinks: usize[];
    stackPushCount: usize;
    stackPopCount: usize;
}
```

```tyrus
import { SccInfo, TarjanSccInfo, tarjanSccInfo } from "tyrus:algorithm";
const info: TarjanSccInfo = tarjanSccInfo([[1], [2], [0, 3], [4], [3]]);
print(info.components); // [[4, 3], [2, 1, 0]]
print(info.componentCount); // 2
print(info.examinedEdgeCount); // 6
print(info.stackOperationCount); // 10
print(info.discoveryIndices); // [0, 1, 2, 3, 4]
print(info.lowLinks); // [0, 1, 2, 3, 4]
print(info.stackPushCount); // 5
print(info.stackPopCount); // 5
```
