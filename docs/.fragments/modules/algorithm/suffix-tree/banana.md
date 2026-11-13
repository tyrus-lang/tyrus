```tyrus
import { suffixTreeUkkonen } from "tyrus:algorithm";

const tree = suffixTreeUkkonen("banana");
const indices = tree.getSuffixArray();

print(indices);  // [5, 3, 1, 0, 4, 2]
```
