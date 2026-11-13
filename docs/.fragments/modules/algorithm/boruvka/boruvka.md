```tyrus
import { boruvka } from "tyrus:algorithm";

const forest: [usize, usize, i32][] = boruvka([[0, 1, 3], [1, 2, 1]], 3);
print(forest.length); // 2
```
