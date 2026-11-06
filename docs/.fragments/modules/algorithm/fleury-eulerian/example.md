```tyrus
import { fleuryEulerian } from "tyrus:algorithm";

const path: usize[]? = fleuryEulerian([[1, 2], [0, 2], [0, 1]]);
if (path) {
    print(path.length); // 4
}
```
