```tyrus
interface FloydWarshallInfo {
    distances: i32?[][];
    kPassCount: usize;
    distanceUpdateCount: usize;
    negativeCycleDetected: boolean;
}
```

```tyrus
import { FloydWarshallInfo, Graph, floydWarshallInfo } from "tyrus:algorithm";

const graph = Graph<i32, i32>();
graph.addEdge(0, 1, 4);
graph.addEdge(0, 2, 1);
graph.addEdge(1, 3, 1);
graph.addEdge(2, 1, 2);
graph.addEdge(2, 3, 5);
const info: FloydWarshallInfo = floydWarshallInfo(graph);
print(info.distances[0][3] ?? -1); // 4
print(info.kPassCount); // 4
print(info.distanceUpdateCount); // 4
print(info.negativeCycleDetected); // false
```
