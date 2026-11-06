```tyrus
import { steinerTree } from "tyrus:algorithm";

const tree: [usize, usize, i32][] = steinerTree([[0, 1, 2], [1, 2, 1]], 3, [0, 2]);
print(tree.length); // 2
```
