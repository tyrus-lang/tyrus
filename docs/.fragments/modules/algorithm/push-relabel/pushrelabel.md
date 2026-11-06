```tyrus
import { pushRelabel } from "tyrus:algorithm";

const result: [i32, [usize, usize, i32][]] = pushRelabel([[0, 2], [0, 0]], 0, 1);
print(result[0]); // 2
```
