```tyrus
import { biconnectedComponents } from "tyrus:algorithm";

const components: [usize, usize][][] = biconnectedComponents([[1, 2], [0, 2, 3], [0, 1], [1]]);
print(components.length); // 2
```
