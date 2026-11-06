```tyrus
import { EdmondsKarpInfo, FlowInfo, edmondsKarpInfo } from "tyrus:algorithm";
const info: EdmondsKarpInfo = edmondsKarpInfo([[0, 3, 2, 0], [0, 0, 1, 2], [0, 0, 0, 3], [0, 0, 0, 0]], 0, 3);
print(info.maxFlow); // 5
print(info.flowEdges); // [[0, 1, 3], [0, 2, 2], [1, 2, 1], [1, 3, 2], [2, 3, 3]]
print(info.augmentationCount); // 5
print(info.examinedEdgeCount); // 16
print(info.bfsRunCount); // 6
print(info.augmentingPathLengths); // [5]
print(info.maxFrontierSize); // 4
```
