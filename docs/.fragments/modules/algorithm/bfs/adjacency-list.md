```tyrus
import { bfs } from "tyrus:algorithm";

const graph: usize[][] = [[1, 2], [3], [3], []];
const order: usize[] = bfs(graph, 0);

print(order);  // [0, 1, 2, 3]
```
