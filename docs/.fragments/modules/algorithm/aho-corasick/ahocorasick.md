```tyrus
import { ahoCorasick } from "tyrus:algorithm";

const matches: [usize, usize][] = ahoCorasick("ababa", ["aba", "ba"]);
print(matches[0][0], matches[0][1]); // 2 0
```
