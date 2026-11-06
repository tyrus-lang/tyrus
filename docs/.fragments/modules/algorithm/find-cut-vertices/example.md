```tyrus
import { findCutVertices } from "tyrus:algorithm";

const cuts: usize[] = findCutVertices([[1, 2], [0, 2, 3], [0, 1], [1]]);
print(cuts.length); // 1
```
