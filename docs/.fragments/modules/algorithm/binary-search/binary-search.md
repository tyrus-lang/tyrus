```tyrus
import { binarySearch } from "tyrus:algorithm";

const index: usize? = binarySearch([1, 2, 4, 7, 9], 7);
print(index ?? 99); // 3
```
