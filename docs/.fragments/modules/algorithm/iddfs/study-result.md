```tyrus
interface IddfsInfo extends TraversalInfo {
    depthLimitsTried: usize[];
    iterationCount: usize;
}
```

```tyrus
import { IddfsInfo, iddfsInfo } from "tyrus:algorithm";

const info: IddfsInfo = iddfsInfo([[1, 2], [3], [3], []], 0, 2);
print(info.visitOrder); // [0, 1, 2, 3]
print(info.depthLimitsTried); // [0, 1, 2]
print(info.iterationCount); // 3
print(info.examinedEdgeCount); // 6
print(info.maxDepth); // 2
```
