```tyrus
import { hierholzerEulerian } from "tyrus:algorithm";

const path: usize[]? = hierholzerEulerian([[1, 2], [0, 2], [0, 1]]);
if (path) {
    print(path.length); // 4
}
```
