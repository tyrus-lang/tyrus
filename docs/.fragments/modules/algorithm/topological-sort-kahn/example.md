```tyrus
import { topologicalSortKahn } from "tyrus:algorithm";

const order: usize[]? = topologicalSortKahn([[1], []]);
if (order) {
    print(order[0]);  // 0
}
```
