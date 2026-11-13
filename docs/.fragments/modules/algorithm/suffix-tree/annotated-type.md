```tyrus
import { suffixTreeUkkonen, SuffixTree } from "tyrus:algorithm";

const tree: SuffixTree = suffixTreeUkkonen("mississippi");
print(tree.getSuffixArray()[0]);   // 10
print(tree.getSuffixArray()[10]);  // 2
```
