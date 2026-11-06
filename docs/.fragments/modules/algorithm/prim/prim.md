```tyrus
import { prim } from "tyrus:algorithm";

const tree: [usize, usize, i32][] = prim([[[1, 2]], [[0, 2], [2, 1]], [[0, 2], [1, 1]]], 0);
print(tree.length); // 2
```
