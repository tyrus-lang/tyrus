```tyrus
import { Counter } from "tyrus:algorithm";

const counts = Counter<i32>();
counts.add(10, 2);
counts.add(20);

for (const pair of counts.mostCommon(2)) {
    print(pair[0]);  // 10, then 20
    print(pair[1]);  // 2, then 1
}
```
