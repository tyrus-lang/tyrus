```tyrus
import { topologicalSortDfs } from "tyrus:algorithm";

const order: usize[]? = topologicalSortDfs([[1], []]);
if (order) {
    print(order[1]);  // 1
}
```
