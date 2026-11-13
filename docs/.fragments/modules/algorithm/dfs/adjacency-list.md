```tyrus
import { dfs } from "tyrus:algorithm";

const graph: usize[][] = [[1, 2], [3], [3], []];
const order: usize[] = dfs(graph, 0);

print(order);  // [0, 1, 3, 2]
```
