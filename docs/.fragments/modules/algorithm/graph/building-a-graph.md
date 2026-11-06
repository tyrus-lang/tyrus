```tyrus
import { Graph } from "tyrus:algorithm";

const g = Graph<string, i32>();
g.addEdge("a", "b", 1);
g.addEdge("b", "c", 2);
g.addEdge("a", "c", 3);
g.addNode("isolated");

print(g.nodeCount());  // 4
print(g.edgeCount());  // 3
print(g.isEmpty());    // false
```
