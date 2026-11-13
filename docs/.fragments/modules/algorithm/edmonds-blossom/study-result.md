```tyrus
interface EdmondsBlossomInfo {
    pairs: [usize, usize][];
    cardinality: usize;
    examinedEdgeCount: usize;
    blossomContractionCount: usize;
    augmentingPathSearchCount: usize;
    augmentationCount: usize;
}
```

```tyrus
import { EdmondsBlossomInfo, MatchingInfo, edmondsBlossomInfo } from "tyrus:algorithm";
const graph: usize[][] = [[1], [0], [3], [2]];
const info: EdmondsBlossomInfo = edmondsBlossomInfo(graph);
print(info.pairs); // [[0, 1], [2, 3]]
print(info.cardinality); // 2
print(info.examinedEdgeCount); // 4
print(info.blossomContractionCount); // 0
print(info.augmentingPathSearchCount); // 4
print(info.augmentationCount); // 2
```
