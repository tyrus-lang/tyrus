```tyrus
import { Graph, bfs } from "tyrus:algorithm";

const graph = Graph<i32, i32>();
graph.addEdge(0, 1, 1);
graph.addEdge(0, 2, 1);
graph.addEdge(1, 3, 1);

const order: usize[] = bfs(graph, 0);
print(order);  // [0, 1, 2, 3]
```
