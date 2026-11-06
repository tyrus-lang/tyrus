```tyrus
import { SegmentTreeLazy } from "tyrus:algorithm";

const st = SegmentTreeLazy([1, 3, 5, 7, 9, 11]);

print(st.query(0, 6));  // 36
print(st.query(2, 5));  // 21
```
