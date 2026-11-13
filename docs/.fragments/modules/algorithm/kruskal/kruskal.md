```tyrus
import { kruskal } from "tyrus:algorithm";

const tree: [usize, usize, i32][] = kruskal([[0, 1, 4], [0, 2, 2], [1, 2, 1]], 3);
print(tree.length); // 2
```
