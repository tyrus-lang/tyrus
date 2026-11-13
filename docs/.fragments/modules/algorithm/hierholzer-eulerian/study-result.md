```tyrus
interface HierholzerEulerianInfo {
    path: usize[]?;
    usedEdgeCount: usize;
    examinedEdgeCount: usize;
    isCircuit: bool;
    stackPushCount: usize;
    stackPopCount: usize;
    joinedCycleCount: usize;
}
```

```tyrus
import { EulerianInfo, HierholzerEulerianInfo, hierholzerEulerianInfo } from "tyrus:algorithm";
const graph: usize[][] = [[1, 2], [0, 2], [0, 1]];
const info: HierholzerEulerianInfo = hierholzerEulerianInfo(graph);
print(info.path ?? []); // [0, 2, 1, 0]
print(info.usedEdgeCount); // 3
print(info.examinedEdgeCount); // 6
print(info.isCircuit); // true
print(info.stackPushCount); // 7
print(info.stackPopCount); // 7
print(info.joinedCycleCount); // 1
```
