```tyrus
import { heavyLightDecomposition } from "tyrus:algorithm";

const single: usize[][] = [[]];
const hld = heavyLightDecomposition(single, 0);

print(hld.queryPath(0, 0));  // [[0, 0]]
```
