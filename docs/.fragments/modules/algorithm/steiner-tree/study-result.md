```tyrus
interface SteinerTreeInfo {
    edges: [usize, usize, i32][];
    totalWeight: i32;
    terminals: usize[];
    evaluatedSubsetCount: usize;
    vertexCount: usize;
    maxTerminalCount: usize;
    withinTerminalLimit: bool;
}
```

```tyrus
import { SteinerTreeInfo, steinerTreeInfo } from "tyrus:algorithm";

const info: SteinerTreeInfo = steinerTreeInfo([[0, 1, 4], [0, 2, 2], [1, 2, 1], [1, 3, 5], [2, 3, 8], [2, 4, 10], [3, 4, 2]], 5, [0, 4]);
print(info.edges); // [[0, 2, 2], [1, 2, 1], [1, 3, 5], [3, 4, 2]]
print(info.totalWeight); // 10
print(info.terminals); // [0, 4]
print(info.evaluatedSubsetCount); // 3
print(info.vertexCount); // 5
print(info.maxTerminalCount); // 20
print(info.withinTerminalLimit); // true
```
