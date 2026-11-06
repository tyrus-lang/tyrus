```tyrus
interface KosarajuSccInfo {
    components: usize[][];
    componentCount: usize;
    examinedEdgeCount: usize;
    stackOperationCount: usize;
    finishingOrder: usize[];
    firstTraversalEdgeCount: usize;
    secondTraversalEdgeCount: usize;
}
```

```tyrus
import { KosarajuSccInfo, SccInfo, kosarajuSccInfo } from "tyrus:algorithm";
const info: KosarajuSccInfo = kosarajuSccInfo([[1], [2], [0, 3], [4], [3]]);
print(info.components); // [[0, 2, 1], [3, 4]]
print(info.componentCount); // 2
print(info.examinedEdgeCount); // 6
print(info.stackOperationCount); // 0
print(info.finishingOrder); // [0, 1, 2, 3, 4]
print(info.firstTraversalEdgeCount); // 6
print(info.secondTraversalEdgeCount); // 6
```
