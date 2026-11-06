```tyrus
import { floydWarshall } from "tyrus:algorithm";

const matrix: (i32?)[][] = [[0, 4, 2], [0, 0, 0], [0, 0, 0]];
const distances = floydWarshall(matrix);
print(distances[0][2] ?? -1); // 2
```
