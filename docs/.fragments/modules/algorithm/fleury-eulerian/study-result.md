```tyrus
interface FleuryEulerianInfo {
    path: usize[]?;
    usedEdgeCount: usize;
    examinedEdgeCount: usize;
    isCircuit: bool;
    bridgeCheckCount: usize;
    avoidedBridgeChoiceCount: usize;
}
```

```tyrus
import { EulerianInfo, FleuryEulerianInfo, fleuryEulerianInfo } from "tyrus:algorithm";
const graph: usize[][] = [[3, 1, 2], [0, 2], [1, 0], [0]];
const info: FleuryEulerianInfo = fleuryEulerianInfo(graph);
print(info.path ?? []); // [0, 1, 2, 0, 3]
print(info.usedEdgeCount); // 4
print(info.examinedEdgeCount); // 5
print(info.isCircuit); // false
print(info.bridgeCheckCount); // 2
print(info.avoidedBridgeChoiceCount); // 1
```
